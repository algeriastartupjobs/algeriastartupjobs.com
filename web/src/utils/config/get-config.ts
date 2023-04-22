import memoize from "lodash/memoize";
import { getStage } from "./get-stage";

const { stageIndex } = getStage();

const _getConfig = () => ({
  api: {
    base_url: [
      `http://${location.hostname}:9090`,
      "https://staging.api.algeriastartupjobs.com",
      "https://production.api.algeriastartupjobs.com",
    ][stageIndex],
  },
});
export const getConfig = memoize(_getConfig);
