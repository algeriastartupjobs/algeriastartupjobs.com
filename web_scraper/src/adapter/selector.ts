import { Source } from "./source";

export interface SelectorInfo {
  value: string;
  from: "innerHTML" | "innerText";
}

export interface Selector {
  title: SelectorInfo;
  description: SelectorInfo;
  poster: SelectorInfo;
}

export const selectors: Record<Source, Selector> = {
  linkedin: {
    title: { value: "h3.sub-nav-cta__header", from: "innerHTML" },
    description: { value: "div.show-more-less-html__markup", from: "innerText" },
    poster: { value: "a.sub-nav-cta__optional-url", from: "innerHTML" },
  },
};

export const getSelectorJS = (source: Source) => {
  const { title, description, poster } = selectors[source];

  return `
    [
      document.querySelector("${title.value}").${title.from},
      document.querySelector("${description.value}").${description.from},
      document.querySelector("${poster.value}").${poster.from}
    ]
  `;
};
