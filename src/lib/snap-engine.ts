import type { PanelState, SnapGuide } from "./types";

const SNAP_THRESHOLD = 8;

interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface SnapResult {
  rect: Rect;
  guides: SnapGuide[];
}

function createRectFromPanel(panel: PanelState): Rect {
  return { x: panel.x, y: panel.y, width: panel.width, height: panel.height };
}

function distance(a: number, b: number): number {
  return Math.abs(a - b);
}

/**
 * 磁性吸附引擎
 * 当被拖面板靠近另一面板边缘或画布边界时，自动对齐贴边
 * @param draggedRect 被拖面板的当前矩形
 * @param otherPanels 其他面板列表
 * @param canvasWidth 画布宽度
 * @param canvasHeight 画布高度
 * @returns 吸附后的矩形和辅助线信息
 */
export function snap(
  draggedRect: Rect,
  otherPanels: PanelState[],
  canvasWidth: number,
  canvasHeight: number
): SnapResult {
  const guides: SnapGuide[] = [];
  let snappedRect = { ...draggedRect };

  const otherRects = otherPanels.map(createRectFromPanel);

  const snapTargets = [
    { position: 0, type: "vertical" as const, targetId: "canvas" },
    { position: canvasWidth, type: "vertical" as const, targetId: "canvas" },
    { position: 0, type: "horizontal" as const, targetId: "canvas" },
    { position: canvasHeight, type: "horizontal" as const, targetId: "canvas" },
  ];

  otherRects.forEach((rect, index) => {
    const panel = otherPanels[index];
    snapTargets.push(
      { position: rect.x, type: "vertical" as const, targetId: panel.id },
      { position: rect.x + rect.width, type: "vertical" as const, targetId: panel.id },
      { position: rect.y, type: "horizontal" as const, targetId: panel.id },
      { position: rect.y + rect.height, type: "horizontal" as const, targetId: panel.id }
    );
  });

  let minDistX = Infinity;
  let snapX: number | null = null;
  let snapXTargetId = "";

  let minDistY = Infinity;
  let snapY: number | null = null;
  let snapYTargetId = "";

  snapTargets.forEach((target) => {
    if (target.type === "vertical") {
      const leftDist = distance(snappedRect.x, target.position);
      const rightDist = distance(snappedRect.x + snappedRect.width, target.position);

      if (leftDist < minDistX && leftDist < SNAP_THRESHOLD) {
        minDistX = leftDist;
        snapX = target.position;
        snapXTargetId = target.targetId;
      }
      if (rightDist < minDistX && rightDist < SNAP_THRESHOLD) {
        minDistX = rightDist;
        snapX = target.position - snappedRect.width;
        snapXTargetId = target.targetId;
      }
    } else {
      const topDist = distance(snappedRect.y, target.position);
      const bottomDist = distance(snappedRect.y + snappedRect.height, target.position);

      if (topDist < minDistY && topDist < SNAP_THRESHOLD) {
        minDistY = topDist;
        snapY = target.position;
        snapYTargetId = target.targetId;
      }
      if (bottomDist < minDistY && bottomDist < SNAP_THRESHOLD) {
        minDistY = bottomDist;
        snapY = target.position - snappedRect.height;
        snapYTargetId = target.targetId;
      }
    }
  });

  if (snapX !== null) {
    snappedRect.x = snapX;
    guides.push({
      type: "vertical",
      position: snapX,
      targetPanelId: snapXTargetId,
    });
  }

  if (snapY !== null) {
    snappedRect.y = snapY;
    guides.push({
      type: "horizontal",
      position: snapY,
      targetPanelId: snapYTargetId,
    });
  }

  return { rect: snappedRect, guides };
}