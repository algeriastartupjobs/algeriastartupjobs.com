export const sources = ["linkedin", "facebook"] as const;
export type Source = (typeof sources)[number];
