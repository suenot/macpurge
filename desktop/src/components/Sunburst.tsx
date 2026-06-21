'use client';

import { useMemo } from 'react';
import * as d3 from 'd3';
import type { UsageNode } from '@/lib/types';

// A laid-out partition node. d3 hangs x0/x1 (angles) and y0/y1 (ring depth)
// off each hierarchy node after running `d3.partition()`.
type ArcNode = d3.HierarchyRectangularNode<UsageNode>;

interface SunburstProps {
  /** Root of the (already pruned) usage tree currently being displayed. */
  root: UsageNode;
  /** Called when a directory arc is clicked, to drill into it. */
  onDrill: (node: UsageNode) => void;
  /** Called on hover with the node under the cursor, or null on leave. */
  onHover?: (node: UsageNode | null) => void;
  /** Called when an arc is clicked (dirs and files alike), for selection. */
  onSelect?: (node: UsageNode) => void;
  /** Called when the dark center hub is clicked (i.e. "go up one level"). */
  onCenter?: () => void;
  /** Pixel width/height of the square SVG. */
  size?: number;
}

// Arcs narrower than this many radians are sub-pixel and culled to keep the
// SVG path count low (the backend already prunes; this is a second guard).
const MIN_ANGLE = 0.003;

/**
 * Hue of a node's depth-1 ancestor, in [0, 1). Every node in the same top-level
 * slice shares this hue, so the sunburst reads as rainbow wedges.
 * Exported so the page's list panel can paint matching color swatches.
 */
export function hueForChildIndex(index: number, total: number): number {
  if (total <= 0) return 0;
  return index / total;
}

// Walk up to the depth-1 ancestor and use its angular midpoint as the hue.
function colorForArc(node: ArcNode): string {
  let ancestor: ArcNode = node;
  while (ancestor.depth > 1 && ancestor.parent) {
    ancestor = ancestor.parent;
  }
  const hue = ((ancestor.x0 + ancestor.x1) / 2) / (2 * Math.PI);
  const base = d3.hsl(d3.interpolateRainbow(hue));
  // Deeper rings get progressively lighter + less saturated so nesting reads.
  const depth = Math.max(node.depth, 1);
  base.s = Math.max(0.25, base.s - (depth - 1) * 0.12);
  base.l = Math.min(0.82, base.l + (depth - 1) * 0.08);
  return base.formatHsl();
}

export default function Sunburst({
  root,
  onDrill,
  onHover,
  onSelect,
  onCenter,
  size = 520,
}: SunburstProps) {
  // Build the layout once per `root` — NOT on hover/tooltip state changes.
  const hierarchy = useMemo(() => {
    const h = d3
      .hierarchy<UsageNode>(root, (d) => d.children)
      .sum((d) => (d.children && d.children.length ? 0 : d.size_bytes))
      .sort((a, b) => (b.value ?? 0) - (a.value ?? 0));
    return d3.partition<UsageNode>().size([2 * Math.PI, h.height + 1])(h);
  }, [root]);

  const radius = size / 2;
  // Map ring index (y0/y1) to pixel radii. The center disc (depth 0) is the hub.
  const ringScale = useMemo(() => {
    const maxRing = hierarchy.height + 1;
    return d3
      .scaleLinear()
      .domain([0, maxRing])
      .range([0, radius]);
  }, [hierarchy, radius]);

  const arc = useMemo(
    () =>
      d3
        .arc<ArcNode>()
        .startAngle((d) => d.x0)
        .endAngle((d) => d.x1)
        .padAngle((d) => Math.min((d.x1 - d.x0) / 2, 0.004))
        .padRadius(radius / 3)
        .innerRadius((d) => ringScale(d.y0))
        .outerRadius((d) => Math.max(ringScale(d.y0), ringScale(d.y1) - 1)),
    [ringScale, radius]
  );

  // Visible arcs: skip the depth-0 root (drawn as the hub) and culled slivers.
  const arcs = useMemo(
    () =>
      hierarchy
        .descendants()
        .filter((d) => d.depth > 0 && d.x1 - d.x0 > MIN_ANGLE),
    [hierarchy]
  );

  const hubRadius = ringScale(1);

  return (
    <svg
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      className="select-none"
      onMouseLeave={() => onHover?.(null)}
    >
      <g transform={`translate(${radius},${radius})`}>
        {arcs.map((node) => {
          const d = arc(node);
          if (!d) return null;
          return (
            <path
              key={node.data.path}
              d={d}
              fill={colorForArc(node)}
              stroke="rgba(10,12,20,0.55)"
              strokeWidth={0.5}
              className="cursor-pointer transition-opacity hover:opacity-80"
              onClick={(e) => {
                e.stopPropagation();
                onSelect?.(node.data);
                if (node.data.is_dir) onDrill(node.data);
              }}
              onMouseEnter={() => onHover?.(node.data)}
            />
          );
        })}

        {/* Center hub doubles as a "go up one level" target. */}
        <circle
          r={Math.max(hubRadius - 1, 6)}
          fill="rgba(15,18,30,0.92)"
          stroke="rgba(255,255,255,0.08)"
          strokeWidth={1}
          className={onCenter ? 'cursor-pointer transition-colors hover:fill-slate-800' : ''}
          onClick={(e) => {
            e.stopPropagation();
            onCenter?.();
          }}
        />
      </g>
    </svg>
  );
}
