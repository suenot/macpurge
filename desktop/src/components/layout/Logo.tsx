const C = 32;
const rad = (d: number) => ((d - 90) * Math.PI) / 180;
const pt = (r: number, a: number) => `${(C + r * Math.cos(rad(a))).toFixed(2)} ${(C + r * Math.sin(rad(a))).toFixed(2)}`;
const sector = (r0: number, r1: number, a0: number, a1: number) =>
  `M${pt(r1, a0)}A${r1} ${r1} 0 ${a1 - a0 > 180 ? 1 : 0} 1 ${pt(r1, a1)}L${pt(r0, a1)}A${r0} ${r0} 0 ${a1 - a0 > 180 ? 1 : 0} 0 ${pt(r0, a0)}Z`;

const RINGS: [number, number, number, string][] = [
  [9, 15, 3, '#8b5cf6'],
  [17, 23, 5, '#f59e0b'],
  [25, 31, 8, '#fde1b8'],
];

const SEGMENTS = RINGS.flatMap(([r0, r1, n, color], ri) =>
  Array.from({ length: n }, (_, i) => {
    const span = 360 / n;
    const a0 = i * span + 1.5, a1 = (i + 1) * span - 1.5, mid = (a0 + a1) / 2;
    return { d: sector(r0, r1, a0, a1), color, key: `${ri}-${i}`, purged: ri > 0 && mid > 36 && mid < 96 };
  }),
).filter((s) => !s.purged);

export function Logo({ size = 32 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 64 64" fill="none" aria-hidden>
      {SEGMENTS.map((s) => <path key={s.key} d={s.d} fill={s.color} />)}
      <circle cx="50" cy="14" r="2.6" fill="#fff" />
    </svg>
  );
}
