/**
 * One floating card for whatever the pointer (or keyboard focus) is on: an item, augment or
 * spell on the Hextech ARAM pages, a chart point. Drawn by `HoverLayer` above everything, so it
 * is never cut off by a scrolling row.
 */
export type HoverStat = { label: string; value: string; tone?: 'up' | 'down' | '' };
export type HoverInfo = {
  icon?: string | null;
  round?: boolean;
  title: string;
  /** A short line under the title (rarity, kind). */
  sub?: string;
  subTone?: 'prismatic' | 'gold' | 'silver' | '';
  stats: HoverStat[];
  note?: string;
};

export const hover = $state({ info: null as HoverInfo | null, rect: null as DOMRect | null });

let hideTimer: ReturnType<typeof setTimeout> | undefined;

export function showHover(el: Element, info: HoverInfo) {
  clearTimeout(hideTimer);
  hover.info = info;
  hover.rect = el.getBoundingClientRect();
}

export function hideHover() {
  clearTimeout(hideTimer);
  // A short grace keeps the card from flickering between neighbours.
  hideTimer = setTimeout(() => {
    hover.info = null;
    hover.rect = null;
  }, 60);
}

/** `use:tip={() => info}`: shows the card on hover and focus. */
export function tip(node: HTMLElement | SVGElement, get: () => HoverInfo | null) {
  let current = get;
  const on = () => {
    const info = current();
    if (info) showHover(node, info);
  };
  node.addEventListener('mouseenter', on);
  node.addEventListener('focus', on);
  node.addEventListener('mouseleave', hideHover);
  node.addEventListener('blur', hideHover);
  return {
    update(next: () => HoverInfo | null) {
      current = next;
    },
    destroy() {
      node.removeEventListener('mouseenter', on);
      node.removeEventListener('focus', on);
      node.removeEventListener('mouseleave', hideHover);
      node.removeEventListener('blur', hideHover);
      hideHover();
    },
  };
}
