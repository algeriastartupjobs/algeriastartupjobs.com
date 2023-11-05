import { Source } from "./source";

export interface Normalizer {
  normalize: (sourceURL: string) => string;
}

const normalizers: Record<Source, Normalizer> = {
  linkedin: {
    normalize: (sourceURL: string) => {
      let jobId = "";

      // /jobs/view/[jobId]
      if (!jobId) {
        const match = sourceURL.match(/\/jobs\/view\/(\d+)/);
        if (match) {
          jobId = match[1];
        }
      }

      // /jobs/collections/recommended/?currentJobId=[jobId]
      if (!jobId) {
        const match = sourceURL.match(/currentJobId=(\d+)/);
        if (match) {
          jobId = match[1];
        }
      }

      const url = `https://www.linkedin.com/jobs/view/${jobId}`;
      return url;
    },
  },
  facebook: {
    normalize: (sourceURL: string) => {
      let postSlug = "";

      // /**/permalink/[permalink]
      if (!postSlug) {
        const match = sourceURL.match(/\/permalink\/(\d+)/);
        if (match) {
          postSlug = match[1];
        }
      }

      return `https://www.facebook.com/${postSlug}`;
    },
  },
};

export const normalizeSourceURL = (source: Source, sourceURL: string) =>
  normalizers[source].normalize(sourceURL);
