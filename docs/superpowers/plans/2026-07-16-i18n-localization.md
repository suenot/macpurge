# i18n Localization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Localize the MacPurge desktop app into 10 languages, with the language auto-detected from macOS and overridable in Settings.

**Architecture:** react-i18next holds the dictionaries and does plural selection. The chosen language lives in the shared Rust config (`~/.macpurge/config.toml`) so the native tray menu can read it too. A single `LanguageProvider` is the only place where i18next, the config, and the tray are wired together; pages only ever call `useTranslation()`.

**Tech Stack:** Next.js 14 (static export, all pages `'use client'`), React 18, TypeScript, Tauri 2, Rust, react-i18next, Vitest (introduced by this plan — the desktop app currently has no tests).

## Global Constraints

- Supported languages, exact codes: `en`, `ru`, `de`, `fr`, `es`, `pt`, `it`, `zh`, `ja`, `ko`. `en` is the base and the fallback.
- `en.json` is the single source of truth for keys. The other nine fall back to it per key.
- Language resolution order: `config.language` → macOS locale → `en`.
- Dictionary keys are flat, namespaced by page: `dev.title`, `settings.language.label`, `sidebar.nav.dashboard`.
- All ten dictionaries are bundled statically. No lazy loading.
- No RTL support. No CLI localization — `cli/` stays English.
- `#[serde(default)]` on every new config field: existing `config.toml` files must load without migration.
- Language names in the switcher are written in their own language (`Русский`, `日本語`, `Deutsch`).
- Never let a config write failure break the UI: apply the language in i18next regardless, log the error.
- Run all frontend commands from `desktop/`.

---

### Task 1: Test infrastructure + locale resolution

The desktop app has no test runner. This task adds Vitest and delivers the first pure module: resolving a browser/system locale string to a supported language code.

**Files:**
- Modify: `desktop/package.json`
- Create: `desktop/vitest.config.ts`
- Create: `desktop/src/lib/i18n/languages.ts`
- Test: `desktop/src/lib/i18n/languages.test.ts`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `type LanguageCode = 'en' | 'ru' | 'de' | 'fr' | 'es' | 'pt' | 'it' | 'zh' | 'ja' | 'ko'`
  - `const SUPPORTED_LANGUAGES: readonly { code: LanguageCode; name: string }[]` — `name` is the endonym.
  - `const DEFAULT_LANGUAGE: LanguageCode` (= `'en'`)
  - `function isLanguageCode(value: unknown): value is LanguageCode`
  - `function resolveLanguage(configured: string | null | undefined, systemLocale: string | null | undefined): LanguageCode`

- [ ] **Step 1: Install Vitest**

```bash
cd desktop
npm install -D vitest@^2 @vitejs/plugin-react@^4 jsdom@^25 @testing-library/react@^16 @testing-library/jest-dom@^6
```

- [ ] **Step 2: Add the test script**

In `desktop/package.json`, add to `"scripts"`:

```json
    "test": "vitest run",
    "test:watch": "vitest"
```

- [ ] **Step 3: Create the Vitest config**

Create `desktop/vitest.config.ts`. The `@/` alias must match `tsconfig.json` or every import breaks.

```ts
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'node:path';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
});
```

- [ ] **Step 4: Write the failing test**

Create `desktop/src/lib/i18n/languages.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import {
  SUPPORTED_LANGUAGES,
  DEFAULT_LANGUAGE,
  isLanguageCode,
  resolveLanguage,
} from './languages';

describe('SUPPORTED_LANGUAGES', () => {
  it('lists exactly the ten supported codes', () => {
    expect(SUPPORTED_LANGUAGES.map((l) => l.code)).toEqual([
      'en', 'ru', 'de', 'fr', 'es', 'pt', 'it', 'zh', 'ja', 'ko',
    ]);
  });

  it('names every language in its own language', () => {
    const names = Object.fromEntries(SUPPORTED_LANGUAGES.map((l) => [l.code, l.name]));
    expect(names.ru).toBe('Русский');
    expect(names.ja).toBe('日本語');
    expect(names.de).toBe('Deutsch');
  });
});

describe('isLanguageCode', () => {
  it('accepts a supported code', () => {
    expect(isLanguageCode('ru')).toBe(true);
  });

  it('rejects anything else', () => {
    expect(isLanguageCode('xx')).toBe(false);
    expect(isLanguageCode(null)).toBe(false);
    expect(isLanguageCode(42)).toBe(false);
  });
});

describe('resolveLanguage', () => {
  it('prefers the configured language over the system locale', () => {
    expect(resolveLanguage('ru', 'de-DE')).toBe('ru');
  });

  it('falls back to the system locale when nothing is configured', () => {
    expect(resolveLanguage(null, 'de-DE')).toBe('de');
  });

  it('matches a system locale on its base language', () => {
    expect(resolveLanguage(null, 'ru-RU')).toBe('ru');
    expect(resolveLanguage(null, 'zh-Hans-CN')).toBe('zh');
    expect(resolveLanguage(null, 'pt-BR')).toBe('pt');
  });

  it('is case-insensitive', () => {
    expect(resolveLanguage(null, 'JA-JP')).toBe('ja');
  });

  it('falls back to English for an unsupported locale', () => {
    expect(resolveLanguage(null, 'sv-SE')).toBe('en');
  });

  it('falls back to English when everything is missing', () => {
    expect(resolveLanguage(null, null)).toBe('en');
    expect(resolveLanguage(undefined, undefined)).toBe('en');
  });

  it('ignores an unsupported configured language and uses the system locale', () => {
    expect(resolveLanguage('xx', 'ja-JP')).toBe('ja');
  });
});
```

- [ ] **Step 5: Run the test to verify it fails**

Run: `cd desktop && npm test -- languages`
Expected: FAIL — `Failed to resolve import "./languages"`.

- [ ] **Step 6: Write the implementation**

Create `desktop/src/lib/i18n/languages.ts`:

```ts
/// Supported UI languages. `en` is the base dictionary and the final fallback.

export type LanguageCode =
  | 'en' | 'ru' | 'de' | 'fr' | 'es' | 'pt' | 'it' | 'zh' | 'ja' | 'ko';

export const DEFAULT_LANGUAGE: LanguageCode = 'en';

// Each language is named in its own language, so a user who picks one by
// accident can still find their way back.
export const SUPPORTED_LANGUAGES: readonly { code: LanguageCode; name: string }[] = [
  { code: 'en', name: 'English' },
  { code: 'ru', name: 'Русский' },
  { code: 'de', name: 'Deutsch' },
  { code: 'fr', name: 'Français' },
  { code: 'es', name: 'Español' },
  { code: 'pt', name: 'Português' },
  { code: 'it', name: 'Italiano' },
  { code: 'zh', name: '中文' },
  { code: 'ja', name: '日本語' },
  { code: 'ko', name: '한국어' },
] as const;

export function isLanguageCode(value: unknown): value is LanguageCode {
  return typeof value === 'string' && SUPPORTED_LANGUAGES.some((l) => l.code === value);
}

/// Resolve the language to use: an explicit choice wins, then the system
/// locale matched on its base language (`zh-Hans-CN` -> `zh`), then English.
export function resolveLanguage(
  configured: string | null | undefined,
  systemLocale: string | null | undefined,
): LanguageCode {
  if (isLanguageCode(configured)) return configured;

  const base = systemLocale?.toLowerCase().split('-')[0];
  if (isLanguageCode(base)) return base;

  return DEFAULT_LANGUAGE;
}
```

- [ ] **Step 7: Run the test to verify it passes**

Run: `cd desktop && npm test -- languages`
Expected: PASS, 10 tests.

- [ ] **Step 8: Commit**

```bash
cd /Users/suenot/projects/sdvg/macpurge
git add desktop/package.json desktop/package-lock.json desktop/vitest.config.ts desktop/src/lib/i18n/
git commit -m "feat(i18n): add Vitest and resolve the UI language from config or system locale"
```

---

### Task 2: Persist the language in the Rust config

The language must live in the shared config because the native tray menu needs it too.

**Files:**
- Modify: `cli/src/config.rs:11-43` (the `Config` struct)
- Modify: `desktop/src-tauri/src/commands/config.rs` (`ConfigResponse`, `get_config`, add `set_language`)
- Modify: `desktop/src-tauri/src/lib.rs:60` (register the command)
- Modify: `desktop/src/lib/types.ts` (`ConfigResponse` type)
- Modify: `desktop/src/lib/tauri.ts` (add `setLanguage`)

**Interfaces:**
- Consumes: `LanguageCode` from Task 1.
- Produces:
  - Rust: `Config.language: Option<String>`; command `set_language(value: Option<String>) -> Result<(), String>`; `ConfigResponse.language: Option<String>`.
  - TS: `async function setLanguage(value: string | null): Promise<void>`; `ConfigResponse.language: string | null`.

- [ ] **Step 1: Add the config field**

In `cli/src/config.rs`, inside `pub struct Config`, after `onboarding_completed` (line 42):

```rust
    /// UI language for the desktop app (BCP 47 base code, e.g. "ru").
    /// None means: follow the system locale. The CLI ignores this field.
    #[serde(default)]
    pub language: Option<String>,
```

- [ ] **Step 2: Write the failing Rust test**

Append to `cli/src/config.rs`:

```rust
#[cfg(test)]
mod language_tests {
    use super::*;

    #[test]
    fn language_defaults_to_none() {
        let config = Config::default();
        assert_eq!(config.language, None);
    }

    #[test]
    fn config_without_language_still_parses() {
        // Existing config.toml files predate this field and must keep loading.
        let toml = "onboarding_completed = true\n";
        let config: Config = toml::from_str(toml).expect("should parse");
        assert_eq!(config.language, None);
        assert!(config.onboarding_completed);
    }

    #[test]
    fn language_round_trips_through_toml() {
        let mut config = Config::default();
        config.language = Some("ru".to_string());
        let serialized = toml::to_string(&config).expect("should serialize");
        let parsed: Config = toml::from_str(&serialized).expect("should parse");
        assert_eq!(parsed.language, Some("ru".to_string()));
    }
}
```

- [ ] **Step 3: Run the test to verify it passes**

Run: `cd cli && cargo test language_tests`
Expected: PASS, 3 tests. If `toml` is not a dev-dependency, add it: `cargo add --dev toml`.

- [ ] **Step 4: Expose the language in `ConfigResponse`**

In `desktop/src-tauri/src/commands/config.rs`, add to `pub struct ConfigResponse` (after `onboarding_completed`):

```rust
    pub language: Option<String>,
```

In `get_config`, add to the returned struct literal:

```rust
        language: config.language.clone(),
```

Every other constructor of `ConfigResponse` in this file needs the same field — the compiler will point at each one.

- [ ] **Step 5: Add the `set_language` command**

In `desktop/src-tauri/src/commands/config.rs`, after `set_onboarding_completed` (line 189), following its exact pattern:

```rust
/// Set the UI language. `None` restores following the system locale.
#[tauri::command]
pub fn set_language(value: Option<String>) -> Result<(), String> {
    let mut config = macpurge::load_config().unwrap_or_default();
    config.language = value;
    macpurge::save_config(&config).map_err(|e| e.to_string())
}
```

- [ ] **Step 6: Register the command**

In `desktop/src-tauri/src/lib.rs`, after line 60 (`commands::config::set_onboarding_completed,`):

```rust
            commands::config::set_language,
```

- [ ] **Step 7: Verify it compiles**

Run: `cd desktop/src-tauri && cargo check`
Expected: no errors.

- [ ] **Step 8: Add the TypeScript binding**

In `desktop/src/lib/types.ts`, add to the `ConfigResponse` interface:

```ts
  language: string | null;
```

In `desktop/src/lib/tauri.ts`, after `setOnboardingCompleted` (line 136):

```ts
export async function setLanguage(value: string | null): Promise<void> {
  return invoke('set_language', { value });
}
```

- [ ] **Step 9: Verify types**

Run: `cd desktop && npx tsc --noEmit`
Expected: `No errors found`.

- [ ] **Step 10: Commit**

```bash
cd /Users/suenot/projects/sdvg/macpurge
git add cli/src/config.rs desktop/src-tauri/src/commands/config.rs desktop/src-tauri/src/lib.rs desktop/src/lib/types.ts desktop/src/lib/tauri.ts
git commit -m "feat(i18n): persist the chosen UI language in the shared config"
```

---

### Task 3: i18next setup, dictionaries, and LanguageProvider

Wires react-i18next into the app. Only `sidebar.*` keys are translated here — this task proves the whole chain end to end, and Task 5 pours in the rest of the strings.

**Files:**
- Modify: `desktop/package.json`
- Create: `desktop/src/locales/en.json` … `ko.json` (10 files)
- Create: `desktop/src/lib/i18n/index.ts`
- Create: `desktop/src/lib/i18n/dictionaries.test.ts`
- Create: `desktop/src/components/LanguageProvider.tsx`
- Modify: `desktop/src/app/layout.tsx`
- Modify: `desktop/src/components/layout/Sidebar.tsx:29-42,110-126`

**Interfaces:**
- Consumes: `resolveLanguage`, `LanguageCode`, `SUPPORTED_LANGUAGES` (Task 1); `getConfig`, `setLanguage` (Task 2).
- Produces:
  - `const i18n` — the configured i18next instance (default export of `lib/i18n/index.ts`).
  - `function LanguageProvider({ children }: { children: React.ReactNode }): JSX.Element`
  - `function useLanguage(): { language: LanguageCode; setLanguage: (code: LanguageCode) => Promise<void> }`

- [ ] **Step 1: Install react-i18next**

```bash
cd desktop
npm install i18next@^23 react-i18next@^15
```

- [ ] **Step 2: Create the English dictionary**

Create `desktop/src/locales/en.json`:

```json
{
  "sidebar": {
    "nav": {
      "dashboard": "Dashboard",
      "disk": "Disk Usage",
      "projects": "Projects",
      "dev": "Dev Artifacts",
      "packages": "Packages",
      "system": "System",
      "apps": "Applications",
      "devtools": "Dev Tools",
      "aiModels": "AI Models",
      "recipes": "Recipes",
      "schedule": "Schedule",
      "settings": "Settings"
    },
    "disk": {
      "type": {
        "system": "System",
        "internal": "Internal",
        "external": "External",
        "network": "Network"
      },
      "used": "{{percent}}% used"
    }
  }
}
```

- [ ] **Step 3: Create the nine other dictionaries**

Create `ru.json`, `de.json`, `fr.json`, `es.json`, `pt.json`, `it.json`, `zh.json`, `ja.json`, `ko.json` in `desktop/src/locales/` with the same key structure, translated. `ru.json`:

```json
{
  "sidebar": {
    "nav": {
      "dashboard": "Обзор",
      "disk": "Диск",
      "projects": "Проекты",
      "dev": "Артефакты сборки",
      "packages": "Пакеты",
      "system": "Система",
      "apps": "Приложения",
      "devtools": "Инструменты",
      "aiModels": "AI-модели",
      "recipes": "Рецепты",
      "schedule": "Расписание",
      "settings": "Настройки"
    },
    "disk": {
      "type": {
        "system": "Системный",
        "internal": "Внутренний",
        "external": "Внешний",
        "network": "Сетевой"
      },
      "used": "{{percent}}% занято"
    }
  }
}
```

Translate the same structure for the remaining eight. Every file must have exactly the keys of `en.json` — Step 5 enforces this.

- [ ] **Step 4: Write the failing dictionary-completeness test**

Create `desktop/src/lib/i18n/dictionaries.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { SUPPORTED_LANGUAGES } from './languages';
import { resources } from './index';

/// Collect every leaf key as a dotted path: { a: { b: 'x' } } -> ['a.b'].
function leafKeys(value: unknown, prefix = ''): string[] {
  if (typeof value !== 'object' || value === null) return [prefix];
  return Object.entries(value).flatMap(([key, child]) =>
    leafKeys(child, prefix ? `${prefix}.${key}` : key),
  );
}

const englishKeys = leafKeys(resources.en.translation).sort();

describe('dictionaries', () => {
  it('ships a dictionary for every supported language', () => {
    for (const { code } of SUPPORTED_LANGUAGES) {
      expect(resources[code], `missing dictionary: ${code}`).toBeDefined();
    }
  });

  it.each(SUPPORTED_LANGUAGES.map((l) => l.code))(
    'dictionary %s has exactly the English keys',
    (code) => {
      expect(leafKeys(resources[code].translation).sort()).toEqual(englishKeys);
    },
  );

  it('has no empty translations', () => {
    for (const { code } of SUPPORTED_LANGUAGES) {
      const empties = leafKeys(resources[code].translation).filter((path) => {
        const value = path.split('.').reduce<any>((o, k) => o?.[k], resources[code].translation);
        return typeof value !== 'string' || value.trim() === '';
      });
      expect(empties, `empty in ${code}`).toEqual([]);
    }
  });
});

describe('russian plurals', () => {
  it('selects three distinct forms', async () => {
    const i18n = (await import('./index')).default;
    await i18n.changeLanguage('ru');
    // Uses the test-only key added in Step 6.
    const one = i18n.t('test.files', { count: 1 });
    const few = i18n.t('test.files', { count: 2 });
    const many = i18n.t('test.files', { count: 5 });
    expect(new Set([one, few, many]).size).toBe(3);
    expect(one).toBe('1 файл');
    expect(few).toBe('2 файла');
    expect(many).toBe('5 файлов');
  });
});
```

- [ ] **Step 5: Add the plural canary keys to the dictionaries**

The plural test needs a key exercising all three Russian forms. This key ships
permanently and is never rendered: it is a canary proving the plural rules stay
wired for every language, and it keeps working after Task 5 rewrites the real
copy. Add to `en.json`:

```json
  "test": {
    "files_one": "{{count}} file",
    "files_other": "{{count}} files"
  }
```

Add to `ru.json`:

```json
  "test": {
    "files_one": "{{count}} файл",
    "files_few": "{{count}} файла",
    "files_many": "{{count}} файлов"
  }
```

Add `test.files_one` / `test.files_other` to the other eight dictionaries, translated. Note: `zh.json` and `ja.json` and `ko.json` have no plural forms — they use `files_other` only, so give them **both** `files_one` and `files_other` with identical text to keep the key sets equal.

- [ ] **Step 6: Run the test to verify it fails**

Run: `cd desktop && npm test -- dictionaries`
Expected: FAIL — `Failed to resolve import "./index"`.

- [ ] **Step 7: Configure i18next**

Create `desktop/src/lib/i18n/index.ts`:

```ts
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import { DEFAULT_LANGUAGE, type LanguageCode } from './languages';

import en from '@/locales/en.json';
import ru from '@/locales/ru.json';
import de from '@/locales/de.json';
import fr from '@/locales/fr.json';
import es from '@/locales/es.json';
import pt from '@/locales/pt.json';
import it from '@/locales/it.json';
import zh from '@/locales/zh.json';
import ja from '@/locales/ja.json';
import ko from '@/locales/ko.json';

// All ten dictionaries are bundled: together they are tens of kilobytes in a
// local app, and lazy loading would only buy a flash of untranslated UI.
export const resources: Record<LanguageCode, { translation: Record<string, unknown> }> = {
  en: { translation: en },
  ru: { translation: ru },
  de: { translation: de },
  fr: { translation: fr },
  es: { translation: es },
  pt: { translation: pt },
  it: { translation: it },
  zh: { translation: zh },
  ja: { translation: ja },
  ko: { translation: ko },
};

i18n.use(initReactI18next).init({
  resources,
  lng: DEFAULT_LANGUAGE,
  fallbackLng: DEFAULT_LANGUAGE,
  interpolation: { escapeValue: false },
  returnEmptyString: false,
  // A missing key falls back to English rather than breaking the build, so new
  // English copy shows through untranslated instead of blank.
  saveMissing: false,
});

if (process.env.NODE_ENV === 'development') {
  i18n.on('missingKey', (lngs, namespace, key) => {
    console.warn(`[i18n] missing key "${key}" for ${lngs}`);
  });
}

export default i18n;
```

- [ ] **Step 8: Run the test to verify it passes**

Run: `cd desktop && npm test -- dictionaries`
Expected: PASS. If a `*_few` / `*_many` key is missing, the Russian plural test names it.

- [ ] **Step 9: Write the failing LanguageProvider test**

Create `desktop/src/components/LanguageProvider.test.tsx`:

```tsx
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { LanguageProvider, useLanguage } from './LanguageProvider';

const getConfig = vi.fn();
const setLanguage = vi.fn();

vi.mock('@/lib/tauri', () => ({
  getConfig: (...args: unknown[]) => getConfig(...args),
  setLanguage: (...args: unknown[]) => setLanguage(...args),
  isTauri: () => true,
}));

function Probe() {
  const { language } = useLanguage();
  return <span data-testid="lang">{language}</span>;
}

describe('LanguageProvider', () => {
  beforeEach(() => {
    getConfig.mockReset();
    setLanguage.mockReset();
  });

  it('applies the language from the config', async () => {
    getConfig.mockResolvedValue({ language: 'ru' });
    render(<LanguageProvider><Probe /></LanguageProvider>);
    await waitFor(() => expect(screen.getByTestId('lang')).toHaveTextContent('ru'));
  });

  it('keeps the UI usable when the config cannot be read', async () => {
    getConfig.mockRejectedValue(new Error('no config'));
    render(<LanguageProvider><Probe /></LanguageProvider>);
    await waitFor(() => expect(screen.getByTestId('lang')).toHaveTextContent('en'));
  });
});
```

Add to `desktop/vitest.config.ts` under `test:`:

```ts
    setupFiles: ['./vitest.setup.ts'],
```

Create `desktop/vitest.setup.ts`:

```ts
import '@testing-library/jest-dom/vitest';
```

- [ ] **Step 10: Run the test to verify it fails**

Run: `cd desktop && npm test -- LanguageProvider`
Expected: FAIL — cannot resolve `./LanguageProvider`.

- [ ] **Step 11: Write the LanguageProvider**

Create `desktop/src/components/LanguageProvider.tsx`. This is the only place where i18next, the config, and the tray meet.

```tsx
'use client';

import { createContext, useCallback, useContext, useEffect, useState } from 'react';
import { getConfig, setLanguage as saveLanguage, isTauri } from '@/lib/tauri';
import { resolveLanguage, DEFAULT_LANGUAGE, type LanguageCode } from '@/lib/i18n/languages';
import i18n from '@/lib/i18n';

interface LanguageContextValue {
  language: LanguageCode;
  setLanguage: (code: LanguageCode) => Promise<void>;
}

const LanguageContext = createContext<LanguageContextValue>({
  language: DEFAULT_LANGUAGE,
  setLanguage: async () => {},
});

export function useLanguage(): LanguageContextValue {
  return useContext(LanguageContext);
}

export function LanguageProvider({ children }: { children: React.ReactNode }) {
  const [language, setLanguageState] = useState<LanguageCode>(DEFAULT_LANGUAGE);

  const apply = useCallback((code: LanguageCode) => {
    setLanguageState(code);
    i18n.changeLanguage(code);
    document.documentElement.lang = code;
  }, []);

  useEffect(() => {
    let mounted = true;

    (async () => {
      let configured: string | null = null;
      if (isTauri()) {
        try {
          configured = (await getConfig()).language;
        } catch (err) {
          // An unreadable config is not a reason to show a broken UI: fall
          // through to the system locale.
          console.error('Failed to read the language from config:', err);
        }
      }
      if (!mounted) return;
      apply(resolveLanguage(configured, navigator.language));
    })();

    return () => { mounted = false; };
  }, [apply]);

  const setLanguage = useCallback(async (code: LanguageCode) => {
    // Apply first: the switcher must work even if persisting fails.
    apply(code);
    if (!isTauri()) return;
    try {
      await saveLanguage(code);
    } catch (err) {
      console.error('Failed to persist the language:', err);
    }
  }, [apply]);

  return (
    <LanguageContext.Provider value={{ language, setLanguage }}>
      {children}
    </LanguageContext.Provider>
  );
}
```

- [ ] **Step 12: Run the test to verify it passes**

Run: `cd desktop && npm test -- LanguageProvider`
Expected: PASS, 2 tests.

- [ ] **Step 13: Wrap the app**

In `desktop/src/app/layout.tsx`, import the provider and wrap the tree. `lang="en"` stays as the server-rendered default; `LanguageProvider` corrects `document.documentElement.lang` on mount.

```tsx
import { LanguageProvider } from '@/components/LanguageProvider';
```

Wrap the contents of `<body>`:

```tsx
      <body className="min-h-screen bg-white dark:bg-dark-bg text-slate-900 dark:text-white bg-gradient-glow selection:bg-brand-500/30">
        <LanguageProvider>
          <div className="flex h-screen">
            <Sidebar />
            <main className="flex-1 overflow-auto p-6">
              {children}
            </main>
          </div>
          <Onboarding />
        </LanguageProvider>
      </body>
```

- [ ] **Step 14: Translate the Sidebar**

In `desktop/src/components/layout/Sidebar.tsx`, replace the hardcoded `label` strings with key references. Change `navItems` (lines 29-42) to carry keys:

```tsx
const navItems = [
  { href: '/', labelKey: 'sidebar.nav.dashboard', icon: LayoutDashboard },
  { href: '/disk', labelKey: 'sidebar.nav.disk', icon: PieChart },
  { href: '/projects', labelKey: 'sidebar.nav.projects', icon: FolderGit2 },
  { href: '/dev', labelKey: 'sidebar.nav.dev', icon: Code2 },
  { href: '/packages', labelKey: 'sidebar.nav.packages', icon: Package },
  { href: '/system', labelKey: 'sidebar.nav.system', icon: Shield },
  { href: '/apps', labelKey: 'sidebar.nav.apps', icon: AppWindow },
  { href: '/devtools', labelKey: 'sidebar.nav.devtools', icon: Wrench },
  { href: '/ai-models', labelKey: 'sidebar.nav.aiModels', icon: BrainCircuit },
  { href: '/recipes', labelKey: 'sidebar.nav.recipes', icon: Sparkles },
  { href: '/schedule', labelKey: 'sidebar.nav.schedule', icon: Clock },
  { href: '/settings', labelKey: 'sidebar.nav.settings', icon: Settings },
];
```

Add the hook inside `Sidebar()`:

```tsx
import { useTranslation } from 'react-i18next';
// ...
  const { t } = useTranslation();
```

Render the label: `<span className="font-medium text-sm tracking-wide">{t(item.labelKey)}</span>`

Replace `diskTypeLabel` (lines 53-60) with a key lookup — delete the function and call `t(`sidebar.disk.type.${disk.disk_type}`)` at its use site (line ~152).

Replace the usage line (~178): `{t('sidebar.disk.used', { percent: usage.toFixed(1) })}`

- [ ] **Step 15: Verify the build and types**

Run: `cd desktop && npx tsc --noEmit && npm run build`
Expected: `No errors found`, build succeeds.

- [ ] **Step 16: Commit**

```bash
cd /Users/suenot/projects/sdvg/macpurge
git add desktop/package.json desktop/package-lock.json desktop/vitest.config.ts desktop/vitest.setup.ts desktop/src/locales/ desktop/src/lib/i18n/ desktop/src/components/LanguageProvider.tsx desktop/src/components/LanguageProvider.test.tsx desktop/src/app/layout.tsx desktop/src/components/layout/Sidebar.tsx
git commit -m "feat(i18n): wire react-i18next through a LanguageProvider and translate the sidebar"
```

---

### Task 4: Locale-aware byte and date formatting

`formatBytes` builds its string by hand and always renders English (`34.90 GB`). It has 65 call sites. The pure core takes a locale; a hook binds the current one so call sites stay short.

**Files:**
- Modify: `desktop/src/lib/types.ts:157-180`
- Create: `desktop/src/lib/format.ts`
- Create: `desktop/src/lib/format.test.ts`
- Create: `desktop/src/lib/useFormat.ts`

**Interfaces:**
- Consumes: `LanguageCode` (Task 1), `useLanguage` (Task 3).
- Produces:
  - `function formatBytes(bytes: number | null | undefined, locale: string): string`
  - `function formatBytesParts(bytes: number | null | undefined, locale: string): { value: string; unit: string }`
  - `function useFormat(): { formatBytes: (b: number | null | undefined) => string; formatBytesParts: (b: number | null | undefined) => { value: string; unit: string }; formatDate: (d: Date | number) => string }`

- [ ] **Step 1: Write the failing test**

Create `desktop/src/lib/format.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { formatBytes, formatBytesParts } from './format';

describe('formatBytes', () => {
  it('formats bytes without decimals', () => {
    expect(formatBytes(512, 'en')).toBe('512 B');
  });

  it('formats larger units with two decimals', () => {
    expect(formatBytes(1024 * 1024 * 1024 * 34.9, 'en')).toBe('34.90 GB');
  });

  it('uses the locale decimal separator', () => {
    // German uses a comma; the English form must not leak into a translated UI.
    expect(formatBytes(1536, 'de')).toBe('1,50 KB');
  });

  it('handles missing values', () => {
    expect(formatBytes(null, 'en')).toBe('0 B');
    expect(formatBytes(undefined, 'en')).toBe('0 B');
    expect(formatBytes(NaN, 'en')).toBe('0 B');
  });

  it('caps at TB', () => {
    expect(formatBytes(1024 ** 5, 'en')).toBe('1024.00 TB');
  });
});

describe('formatBytesParts', () => {
  it('splits value from unit', () => {
    expect(formatBytesParts(1024 * 1024, 'en')).toEqual({ value: '1.00', unit: 'MB' });
  });

  it('does not depend on the separator between value and unit', () => {
    // The old implementation split formatBytes() on a space, which breaks for
    // locales that format with a non-breaking space.
    const parts = formatBytesParts(1536, 'de');
    expect(parts.value).toBe('1,50');
    expect(parts.unit).toBe('KB');
  });

  it('handles missing values', () => {
    expect(formatBytesParts(null, 'en')).toEqual({ value: '0', unit: 'B' });
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd desktop && npm test -- format`
Expected: FAIL — cannot resolve `./format`.

- [ ] **Step 3: Write the implementation**

Create `desktop/src/lib/format.ts`:

```ts
/// Locale-aware size and date formatting. Pure functions: the locale is a
/// parameter, never module state, so they stay testable. Components get the
/// current locale bound for them by `useFormat`.

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'] as const;

interface BytesParts {
  value: string;
  unit: string;
}

/// Split a byte count into a scaled number and its unit. Both public
/// formatters build on this, so they can never disagree.
function splitBytes(bytes: number | null | undefined): { size: number; unitIndex: number } {
  if (bytes === undefined || bytes === null || isNaN(bytes)) {
    return { size: 0, unitIndex: 0 };
  }

  let unitIndex = 0;
  let size = bytes;

  while (size >= 1024 && unitIndex < UNITS.length - 1) {
    size /= 1024;
    unitIndex++;
  }

  return { size, unitIndex };
}

export function formatBytesParts(bytes: number | null | undefined, locale: string): BytesParts {
  const { size, unitIndex } = splitBytes(bytes);
  const digits = unitIndex === 0 ? 0 : 2;

  const value = new Intl.NumberFormat(locale, {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(size);

  return { value, unit: UNITS[unitIndex] };
}

export function formatBytes(bytes: number | null | undefined, locale: string): string {
  const { value, unit } = formatBytesParts(bytes, locale);
  return `${value} ${unit}`;
}

export function formatDate(date: Date | number, locale: string): string {
  return new Intl.DateTimeFormat(locale, { dateStyle: 'medium' }).format(date);
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd desktop && npm test -- format`
Expected: PASS, 8 tests.

- [ ] **Step 5: Create the hook**

Create `desktop/src/lib/useFormat.ts`:

```ts
'use client';

import { useMemo } from 'react';
import { useLanguage } from '@/components/LanguageProvider';
import { formatBytes, formatBytesParts, formatDate } from './format';

/// Binds the current language to the pure formatters so call sites stay short.
export function useFormat() {
  const { language } = useLanguage();

  return useMemo(() => ({
    formatBytes: (bytes: number | null | undefined) => formatBytes(bytes, language),
    formatBytesParts: (bytes: number | null | undefined) => formatBytesParts(bytes, language),
    formatDate: (date: Date | number) => formatDate(date, language),
  }), [language]);
}
```

- [ ] **Step 6: Re-point the old formatters**

In `desktop/src/lib/types.ts`, delete `formatBytes` (lines 157-172) and `formatBytesParts` (lines 174-180). Replace them with a re-export that keeps the 65 existing call sites compiling until they migrate:

```ts
// Deprecated: use `useFormat()` in components. These keep the English locale
// and exist only so call sites can migrate one page at a time.
import { formatBytes as formatBytesWithLocale, formatBytesParts as formatBytesPartsWithLocale } from './format';

/** @deprecated Use `useFormat().formatBytes` — this one is always English. */
export function formatBytes(bytes: number | undefined | null): string {
  return formatBytesWithLocale(bytes, 'en');
}

/** @deprecated Use `useFormat().formatBytesParts` — this one is always English. */
export function formatBytesParts(bytes: number | undefined | null): { value: string; unit: string } {
  return formatBytesPartsWithLocale(bytes, 'en');
}
```

- [ ] **Step 7: Verify types and tests**

Run: `cd desktop && npx tsc --noEmit && npm test`
Expected: `No errors found`; all tests pass.

- [ ] **Step 8: Commit**

```bash
cd /Users/suenot/projects/sdvg/macpurge
git add desktop/src/lib/format.ts desktop/src/lib/format.test.ts desktop/src/lib/useFormat.ts desktop/src/lib/types.ts
git commit -m "feat(i18n): make byte formatting locale-aware and drop the fragile split"
```

---

### Task 5: Extract the page strings

The bulk of the work: move every hardcoded string in 15 remaining files into `en.json`, and swap `formatBytes` imports for `useFormat()`.

**This task is split per page. The files do not overlap, so the sub-tasks run in parallel.** Each sub-task is independently reviewable and independently committable.

**Interfaces:**
- Consumes: `useTranslation` from `react-i18next`, `useFormat` (Task 4), key conventions from Task 3.
- Produces: new `en.json` namespaces, one per page.

**Rules that apply to every sub-task below:**

1. Add keys under a namespace named after the page: `projects.*`, `settings.*`, `disk.*`, `dev.*`, `packages.*`, `dashboard.*`, `schedule.*`, `apps.*`, `onboarding.*`, `aiModels.*`, `system.*`, `devtools.*`, `recipes.*`, `confirmDialog.*`, `projectRow.*`, `sunburst.*`.
2. Only `en.json` changes. Task 6 translates the other nine. **Do not** hand-translate here — the dictionary test would fail on a key mismatch, so instead: after adding keys to `en.json`, add the same keys with the **English text** to the other nine files. Task 6 replaces that text.
3. Counted nouns use i18next plurals (`key_one` / `key_other`), never `${n} item(s)` or manual `n === 1 ? …`.
4. Interpolate values, never concatenate translated fragments: `t('dev.found', { count })`, not `t('dev.found') + count`.
5. Swap `import { formatBytes } from '@/lib/types'` for `const { formatBytes } = useFormat()` inside the component.
6. Do not translate: file paths, package names, brand names (`Homebrew`, `node_modules`, `MacPurge`), CLI commands.
7. After each sub-task: `npx tsc --noEmit && npm test && npm run build`.

- [ ] **Sub-task 5a: `src/app/projects/page.tsx` (1174 lines) + `src/components/ProjectRow.tsx` (260 lines)**

The two files share the `projects.*` namespace, so they move together.

- [ ] **Sub-task 5b: `src/app/settings/page.tsx` (809 lines)**

Namespace `settings.*`. Leave room for the switcher: Task 7 adds `settings.language.*`.

- [ ] **Sub-task 5c: `src/app/disk/page.tsx` (608 lines) + `src/components/Sunburst.tsx` (151 lines)**

Namespaces `disk.*`, `sunburst.*`. Includes `No targets available` (line 272).

- [ ] **Sub-task 5d: `src/app/dev/page.tsx` (542 lines)**

Namespace `dev.*`. Includes `No Artifacts Found`, `Scanning your project directories...`, `Clean Selected (0 B)` — the last interpolates a formatted size.

- [ ] **Sub-task 5e: `src/app/packages/page.tsx` (487 lines)**

Namespace `packages.*`.

- [ ] **Sub-task 5f: `src/app/page.tsx` (347 lines, Dashboard)**

Namespace `dashboard.*`.

- [ ] **Sub-task 5g: `src/app/schedule/page.tsx` (325 lines)**

Namespace `schedule.*`. Weekday and time strings use `Intl.DateTimeFormat`, not hardcoded English names.

- [ ] **Sub-task 5h: `src/app/apps/page.tsx` (286 lines)**

Namespace `apps.*`.

- [ ] **Sub-task 5i: `src/components/Onboarding.tsx` (273 lines)**

Namespace `onboarding.*`. This is the first screen a new user sees, so it must be translated before first paint — verify no English flash on a `ru` config.

- [ ] **Sub-task 5j: `src/app/ai-models/page.tsx` (268 lines)**

Namespace `aiModels.*`.

- [ ] **Sub-task 5k: `src/app/system/page.tsx` (250 lines)**

Namespace `system.*`.

- [ ] **Sub-task 5l: `src/app/devtools/page.tsx` (236 lines)**

Namespace `devtools.*`.

- [ ] **Sub-task 5m: `src/app/recipes/page.tsx` (225 lines)**

Namespace `recipes.*`.

- [ ] **Sub-task 5n: `src/components/ui/ConfirmDialog.tsx` (83 lines)**

Namespace `confirmDialog.*`. Default button labels (`Cancel`, `Confirm`) become defaults from the dictionary, still overridable by props.

- [ ] **Step: Remove the deprecated shims**

Once no file imports `formatBytes` from `@/lib/types`, delete both deprecated functions from `types.ts` (added in Task 4, Step 6).

Verify none remain:

```bash
cd desktop && grep -rn "formatBytes\|formatBytesParts" src/ --include=*.tsx | grep -v useFormat
```

Expected: no output.

- [ ] **Step: Commit**

```bash
cd /Users/suenot/projects/sdvg/macpurge
git add desktop/src
git commit -m "refactor(i18n): move the remaining UI strings into the dictionary"
```

---

### Task 6: Translate the dictionaries into nine languages

At this point `en.json` holds every key, and the other nine hold English placeholder text.

**Files:**
- Modify: `desktop/src/locales/{ru,de,fr,es,pt,it,zh,ja,ko}.json`

**Interfaces:**
- Consumes: the complete `en.json` from Task 5.
- Produces: nine translated dictionaries with identical key sets.

- [ ] **Step 1: Translate, one language per sub-task**

Nine independent sub-tasks — the files do not overlap. For each: translate every value in `<lang>.json` from the English text, keeping keys byte-identical.

Rules:
- Keep `{{placeholders}}` exactly as they are, including spelling.
- Add the plural categories the language actually needs: `ru` needs `_one`/`_few`/`_many`; `zh`, `ja`, `ko` need only `_other` — but keep `_one` too, with identical text, so the key sets stay equal.
- Do not translate brand names, file paths, or CLI commands.
- Keep UI copy short: German and Russian run 30% longer than English and will overflow buttons. Prefer the shorter phrasing.
- Russian text follows the repo rule: "е" instead of "ё", straight quotes only, no transliterated English.

- [ ] **Step 2: Verify the key sets still match**

Run: `cd desktop && npm test -- dictionaries`
Expected: PASS — proves every language has exactly the English keys and no empty strings.

- [ ] **Step 3: Commit**

```bash
cd /Users/suenot/projects/sdvg/macpurge
git add desktop/src/locales
git commit -m "feat(i18n): translate the UI into nine languages"
```

---

### Task 7: Language switcher in Settings

**Files:**
- Modify: `desktop/src/app/settings/page.tsx`
- Modify: `desktop/src/locales/*.json` (add `settings.language.*`)

**Interfaces:**
- Consumes: `useLanguage` (Task 3), `SUPPORTED_LANGUAGES` (Task 1).
- Produces: nothing downstream.

- [ ] **Step 1: Add the keys**

To `en.json`:

```json
    "language": {
      "label": "Language",
      "description": "The app language. Follows macOS by default."
    }
```

Add the same keys to the other nine, translated.

- [ ] **Step 2: Build the switcher**

In `desktop/src/app/settings/page.tsx`, add a themed dropdown modelled on the disk picker (`src/app/disk/page.tsx:269-300`) — a `useState` open flag plus an absolutely-positioned panel. Do not use a native `<select>`: commit 90ccecc replaced one because it cannot be themed.

```tsx
const { language, setLanguage } = useLanguage();
const [languageOpen, setLanguageOpen] = useState(false);
const current = SUPPORTED_LANGUAGES.find((l) => l.code === language);
```

The panel lists `SUPPORTED_LANGUAGES` by `name` (the endonym — never translate these), marks the active one with a `Check` icon, and calls:

```tsx
onClick={() => {
  setLanguage(l.code);
  setLanguageOpen(false);
}}
```

- [ ] **Step 3: Verify**

Run: `cd desktop && npx tsc --noEmit && npm test && npm run build`
Expected: all pass.

- [ ] **Step 4: Commit**

```bash
cd /Users/suenot/projects/sdvg/macpurge
git add desktop/src/app/settings/page.tsx desktop/src/locales
git commit -m "feat(i18n): add the language switcher to Settings"
```

---

### Task 8: Translate the tray menu

Four strings on the native side. The tray is built by Rust and cannot read the JS dictionary, so it carries its own small table — four strings do not justify a shared format and a codegen step.

**Files:**
- Modify: `desktop/src-tauri/src/tray.rs:24-33`
- Create: `desktop/src-tauri/src/tray_i18n.rs`
- Modify: `desktop/src-tauri/src/lib.rs` (declare the module, register `rebuild_tray_menu`)
- Modify: `desktop/src/lib/tauri.ts`
- Modify: `desktop/src/components/LanguageProvider.tsx`

**Interfaces:**
- Consumes: `Config.language` (Task 2).
- Produces:
  - Rust: `fn tray_labels(language: &str) -> TrayLabels`, command `rebuild_tray_menu(app: tauri::AppHandle) -> Result<(), String>`.
  - TS: `async function rebuildTrayMenu(): Promise<void>`.

- [ ] **Step 1: Write the failing Rust test**

Create `desktop/src-tauri/src/tray_i18n.rs`:

```rust
//! Translations for the native tray menu. The tray is built in Rust and cannot
//! read the JS dictionary, so it keeps its own table. Four strings do not
//! justify a shared format and a codegen step.

pub struct TrayLabels {
    pub show: &'static str,
    pub scan: &'static str,
    pub devtools: &'static str,
    pub quit: &'static str,
}

const EN: TrayLabels = TrayLabels {
    show: "Show MacPurge",
    scan: "Quick Scan",
    devtools: "Toggle DevTools",
    quit: "Quit",
};

/// Labels for a language code, falling back to English for anything unknown.
pub fn tray_labels(language: &str) -> TrayLabels {
    match language {
        "ru" => TrayLabels {
            show: "Показать MacPurge",
            scan: "Быстрое сканирование",
            devtools: "Инструменты разработчика",
            quit: "Выйти",
        },
        "de" => TrayLabels {
            show: "MacPurge anzeigen",
            scan: "Schnellscan",
            devtools: "DevTools umschalten",
            quit: "Beenden",
        },
        "fr" => TrayLabels {
            show: "Afficher MacPurge",
            scan: "Analyse rapide",
            devtools: "Basculer DevTools",
            quit: "Quitter",
        },
        "es" => TrayLabels {
            show: "Mostrar MacPurge",
            scan: "Análisis rápido",
            devtools: "Alternar DevTools",
            quit: "Salir",
        },
        "pt" => TrayLabels {
            show: "Mostrar MacPurge",
            scan: "Verificação rápida",
            devtools: "Alternar DevTools",
            quit: "Sair",
        },
        "it" => TrayLabels {
            show: "Mostra MacPurge",
            scan: "Scansione rapida",
            devtools: "Attiva/disattiva DevTools",
            quit: "Esci",
        },
        "zh" => TrayLabels {
            show: "显示 MacPurge",
            scan: "快速扫描",
            devtools: "切换开发者工具",
            quit: "退出",
        },
        "ja" => TrayLabels {
            show: "MacPurge を表示",
            scan: "クイックスキャン",
            devtools: "DevTools を切り替え",
            quit: "終了",
        },
        "ko" => TrayLabels {
            show: "MacPurge 표시",
            scan: "빠른 검사",
            devtools: "DevTools 전환",
            quit: "종료",
        },
        _ => EN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_translated_labels() {
        assert_eq!(tray_labels("ru").quit, "Выйти");
        assert_eq!(tray_labels("ja").quit, "終了");
    }

    #[test]
    fn falls_back_to_english() {
        assert_eq!(tray_labels("xx").quit, "Quit");
        assert_eq!(tray_labels("").quit, "Quit");
    }

    #[test]
    fn covers_every_supported_language() {
        // Keep in step with SUPPORTED_LANGUAGES in src/lib/i18n/languages.ts.
        for code in ["en", "ru", "de", "fr", "es", "pt", "it", "zh", "ja", "ko"] {
            let labels = tray_labels(code);
            assert!(!labels.quit.is_empty(), "no labels for {code}");
            if code != "en" {
                assert_ne!(labels.quit, "Quit", "{code} falls back to English");
            }
        }
    }
}
```

- [ ] **Step 2: Declare the module**

In `desktop/src-tauri/src/lib.rs`, next to the other `mod` declarations:

```rust
mod tray_i18n;
```

- [ ] **Step 3: Run the test**

Run: `cd desktop/src-tauri && cargo test tray_i18n`
Expected: PASS, 3 tests.

- [ ] **Step 4: Use the labels when building the tray**

In `desktop/src-tauri/src/tray.rs`, replace the hardcoded literals (lines 26-30):

```rust
    let language = macpurge::load_config()
        .ok()
        .and_then(|c| c.language)
        .unwrap_or_default();
    let labels = crate::tray_i18n::tray_labels(&language);

    let show = MenuItem::with_id(app, "show", labels.show, true, None::<&str>)?;
    let scan = MenuItem::with_id(app, "scan", labels.scan, true, None::<&str>)?;
    let devtools = MenuItem::with_id(app, "devtools", labels.devtools, true, None::<&str>)?;
    let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", labels.quit, true, None::<&str>)?;
```

- [ ] **Step 5: Add the rebuild command**

Extract the menu construction from `setup_tray` into a helper both it and the command call, then add to `tray.rs`:

```rust
/// Rebuild the tray menu, picking up the current language from config.
#[tauri::command]
pub fn rebuild_tray_menu(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    let language = macpurge::load_config()
        .ok()
        .and_then(|c| c.language)
        .unwrap_or_default();
    let labels = crate::tray_i18n::tray_labels(&language);

    let menu = build_menu(&app, &labels).map_err(|e| e.to_string())?;
    let tray = app.tray_by_id("main-tray").ok_or("tray not found")?;
    tray.set_menu(Some(menu)).map_err(|e| e.to_string())
}
```

Register it in `lib.rs` alongside the other handlers:

```rust
            tray::rebuild_tray_menu,
```

- [ ] **Step 6: Call it from the provider**

In `desktop/src/lib/tauri.ts`:

```ts
export async function rebuildTrayMenu(): Promise<void> {
  return invoke('rebuild_tray_menu');
}
```

In `LanguageProvider.tsx`, extend `setLanguage` — the tray is cosmetic, so its failure must not surface:

```tsx
    try {
      await saveLanguage(code);
      await rebuildTrayMenu();
    } catch (err) {
      console.error('Failed to persist the language:', err);
    }
```

- [ ] **Step 7: Verify**

Run: `cd desktop/src-tauri && cargo test && cd .. && npx tsc --noEmit && npm test`
Expected: all pass.

- [ ] **Step 8: Commit**

```bash
cd /Users/suenot/projects/sdvg/macpurge
git add desktop/src-tauri/src/tray_i18n.rs desktop/src-tauri/src/tray.rs desktop/src-tauri/src/lib.rs desktop/src/lib/tauri.ts desktop/src/components/LanguageProvider.tsx
git commit -m "feat(i18n): translate the tray menu and rebuild it on language change"
```

---

### Task 9: Manual verification and release

**Files:** none — verification only.

- [ ] **Step 1: Full check**

Run: `cd desktop && npm test && npx tsc --noEmit && npm run build && cd src-tauri && cargo test`
Expected: everything passes.

- [ ] **Step 2: Build and install**

```bash
cd desktop && npm run tauri build
rm -rf /Applications/MacPurge.app
cp -R src-tauri/target/release/bundle/macos/MacPurge.app /Applications/
xattr -dr com.apple.quarantine /Applications/MacPurge.app
open -a /Applications/MacPurge.app
```

- [ ] **Step 3: Verify in the running app**

Confirm each, and report what was actually observed:
- The app opens in the macOS language (or English if that language is unsupported).
- Switching the language in Settings retranslates the UI immediately, with no restart.
- The tray menu changes language too (click the tray icon).
- Restarting the app keeps the chosen language.
- Sizes render in the local format: `1,50 KB` in German, `1.50 KB` in English.
- Russian counts read correctly: `1 файл`, `2 файла`, `5 файлов`.
- No English text remains on a non-English language. Walk every page in the sidebar.
- Long German and Russian labels do not overflow their buttons.

- [ ] **Step 4: Push**

```bash
cd /Users/suenot/projects/sdvg/macpurge && git push
```
