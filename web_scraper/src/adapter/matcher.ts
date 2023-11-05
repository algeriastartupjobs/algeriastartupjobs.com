import { Source } from "./source";

export interface Matcher {
  source: Source;
  match: (sourceURL: string) => boolean;
}

export const matchers: Matcher[] = [
  {
    source: "linkedin",
    match: (sourceURL: string) => {
      const url = new URL(sourceURL);

      return [url.hostname.endsWith("linkedin.com")].some(Boolean);
    },
  },
];

export const matchSourceURL = (sourceURL: string) =>
  matchers.find(({ match }) => match(sourceURL))?.source;
