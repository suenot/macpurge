use anyhow::{Context, Result};
use std::process::Command;

use crate::cleaners::{list_package_manager_caches, ApplicationScanner};
use crate::reporters::generate_claude_report;
use crate::scanner::{get_all_disks, DirectoryScanner, ScanOptions};

/// Claude Code CLI integration
pub struct ClaudeIntegration;

impl ClaudeIntegration {
    /// Check if Claude CLI is available
    pub fn is_available() -> bool {
        Command::new("which")
            .arg("claude")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Ask Claude a question with context about the system
    pub fn ask(question: &str) -> Result<String> {
        let context = Self::generate_context()?;

        let prompt = format!(
            "{}\n\n---\n\nВопрос пользователя: {}",
            context, question
        );

        Self::run_claude(&prompt)
    }

    /// Ask Claude with custom context
    pub fn ask_with_context(question: &str, context: &str) -> Result<String> {
        let prompt = format!(
            "{}\n\n---\n\nВопрос пользователя: {}",
            context, question
        );

        Self::run_claude(&prompt)
    }

    /// Run Claude CLI with a prompt
    fn run_claude(prompt: &str) -> Result<String> {
        if !Self::is_available() {
            anyhow::bail!(
                "Claude CLI not found. Please install it from https://claude.ai/download"
            );
        }

        let output = Command::new("claude")
            .arg("-p")
            .arg(prompt)
            .output()
            .context("Failed to execute claude command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Claude CLI failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Generate system context for Claude
    pub fn generate_context() -> Result<String> {
        let mut context = String::new();

        context.push_str("# Контекст системы macOS\n\n");

        // Disk info
        if let Ok(disks) = get_all_disks() {
            context.push_str("## Диски\n\n");
            for disk in &disks {
                context.push_str(&format!(
                    "- {}: {} использовано из {} ({:.1}% занято)\n",
                    disk.name,
                    disk.used_human(),
                    disk.total_human(),
                    disk.usage_percent
                ));
            }
            context.push('\n');
        }

        // Package manager caches
        context.push_str("## Кеши пакетных менеджеров\n\n");
        for (manager, size, installed) in list_package_manager_caches() {
            if size > 0 {
                let status = if installed { "установлен" } else { "не установлен" };
                context.push_str(&format!(
                    "- {}: {} ({})\n",
                    manager.display_name(),
                    crate::utils::format_size(size),
                    status
                ));
            }
        }
        context.push('\n');

        // Application remnants summary
        if let Ok(remnants) = ApplicationScanner::find_remnants() {
            if !remnants.is_empty() {
                let total_size: u64 = remnants.iter().map(|r| r.total_size).sum();
                context.push_str("## Остатки приложений\n\n");
                context.push_str(&format!(
                    "Найдено {} остатков от удалённых приложений, общий размер: {}\n\n",
                    remnants.len(),
                    crate::utils::format_size(total_size)
                ));

                // Top 10 remnants
                context.push_str("Топ 10 по размеру:\n");
                for remnant in remnants.iter().take(10) {
                    context.push_str(&format!(
                        "- {}: {}\n",
                        remnant.app_name,
                        remnant.size_human()
                    ));
                }
                context.push('\n');
            }
        }

        Ok(context)
    }

    /// Generate full analysis report for Claude
    pub fn generate_analysis_report(scan_path: Option<&std::path::Path>) -> Result<String> {
        let disk_info = get_all_disks().ok();

        let scan_result = if let Some(path) = scan_path {
            let scanner = DirectoryScanner::new(ScanOptions {
                dev_mode: true,
                show_progress: false,
                ..Default::default()
            });
            Some(scanner.scan(path)?)
        } else {
            // Scan home directory by default
            let home = dirs::home_dir().context("Could not find home directory")?;
            let scanner = DirectoryScanner::new(ScanOptions {
                dev_mode: true,
                show_progress: false,
                max_depth: Some(5),
                ..Default::default()
            });
            scanner.scan(&home).ok()
        };

        generate_claude_report(
            scan_result.as_ref(),
            disk_info.as_deref(),
        )
    }

    /// Ask Claude for cleanup recommendations
    pub fn get_recommendations() -> Result<String> {
        let report = Self::generate_analysis_report(None)?;

        let question = "На основе этого отчёта, что ты рекомендуешь почистить в первую очередь? \
                        Расставь приоритеты по соотношению освобождаемого места к риску. \
                        Дай конкретные команды для выполнения.";

        Self::ask_with_context(question, &report)
    }

    /// Interactive cleanup session with Claude
    pub fn interactive_cleanup() -> Result<()> {
        use std::io::{self, BufRead, Write};

        println!("Запуск интерактивной сессии с Claude...\n");

        if !Self::is_available() {
            anyhow::bail!(
                "Claude CLI не найден. Установите его с https://claude.ai/download"
            );
        }

        let context = Self::generate_context()?;
        println!("Контекст системы загружен. Задавайте вопросы (введите 'exit' для выхода):\n");

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            print!("> ");
            stdout.flush()?;

            let mut input = String::new();
            stdin.lock().read_line(&mut input)?;

            let input = input.trim();
            if input.is_empty() {
                continue;
            }

            if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                println!("До свидания!");
                break;
            }

            match Self::ask_with_context(input, &context) {
                Ok(response) => {
                    println!("\n{}\n", response);
                }
                Err(e) => {
                    eprintln!("Ошибка: {}\n", e);
                }
            }
        }

        Ok(())
    }
}

/// Quick functions for common Claude queries

/// Ask what can be cleaned
pub fn ask_what_to_clean() -> Result<String> {
    ClaudeIntegration::ask("Что можно почистить на этом Mac? Какие файлы занимают больше всего места и безопасны для удаления?")
}

/// Ask for disk space analysis
pub fn ask_disk_analysis() -> Result<String> {
    ClaudeIntegration::ask("Проанализируй использование диска и дай рекомендации по освобождению места.")
}

/// Ask about specific directory
pub fn ask_about_directory(path: &std::path::Path) -> Result<String> {
    let scanner = DirectoryScanner::new(ScanOptions {
        dev_mode: true,
        show_progress: false,
        ..Default::default()
    });

    let result = scanner.scan(path)?;
    let report = generate_claude_report(Some(&result), None)?;

    let question = format!(
        "Проанализируй директорию {} и скажи, что можно безопасно удалить.",
        path.display()
    );

    ClaudeIntegration::ask_with_context(&question, &report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_context() {
        let context = ClaudeIntegration::generate_context();
        assert!(context.is_ok());
        let context = context.unwrap();
        assert!(context.contains("Контекст системы"));
    }
}
