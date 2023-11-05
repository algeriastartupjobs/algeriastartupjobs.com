import { getStateActions } from "src/state";
import * as Sentry from "@sentry/react";
import { ImportStatus, initialStateForImportStatusPage } from "./state";
import { createFetchStream } from "src/utils/fetch/create-fetch-stream";
import { onceAtATime } from "src/utils/concurrency/once-at-a-time";
import { getBrowserRouter } from "src/components/router-provider";
import { CREATE_POST_PAGE_URL } from "src/utils/urls/common";
import { viewTransitionSubscribeOnce } from "src/utils/animation/view-transition";
import { ANIMATION_DURATION } from "src/utils/animation/duration";
import { initialStateForImportPage } from "../import/state";
import { sleep } from "src/utils/time/sleep";
import { STAY_AT_COMPLETED_STATUS_FOR_MS } from "./const";
import { initialStateForCreatePostPage } from "../create-post/state";

type ImportStatusResponse =
  | {
      status: Exclude<ImportStatus, "Completed">;
      title?: never;
      description?: never;
      poster?: never;
    }
  | {
      status: Extract<ImportStatus, "Completed">;
      title: string;
      description: string;
      poster: string;
    };

const _fetchImportStatusForURL = async (url: string): Promise<void> => {
  const { importStatusPage, importPage, createPostPage } = getStateActions();
  const initialLocation = getBrowserRouter().state.location;
  try {
    const { listen, close } = createFetchStream<ImportStatusResponse>({
      url: `/imported_content/status?url=${url}`,
    });

    let response: ImportStatusResponse | null = null;
    // @TODO-ZM: handle network errors
    while ((response = await listen())) {
      if (initialLocation !== getBrowserRouter().state.location) {
        importStatusPage.overwrite(initialStateForImportStatusPage);
        close();
        return;
      }
      importStatusPage.set({ status: response.status });
      if (response.status === "Completed") {
        close();

        await sleep(STAY_AT_COMPLETED_STATUS_FOR_MS);
        const { title, description, poster } = response;
        createPostPage.overwrite({
          ...initialStateForCreatePostPage,
          title,
          compact: false,
          post_description: description,
          poster_name: poster,
        });

        setTimeout(() => {
          viewTransitionSubscribeOnce(() => {
            importStatusPage.overwrite(initialStateForImportStatusPage);
            importPage.overwrite(initialStateForImportPage);
          });
          getBrowserRouter().navigate(CREATE_POST_PAGE_URL);
        }, ANIMATION_DURATION);
      }
    }
  } catch (error) {
    importStatusPage.set({ status: "Failed" });
    // @TODO-ZM: use Logger abstraction instead of console.log
    console.log("Error while fetching import status", error);
    Sentry.captureException(error, { tags: { type: "WEB_FETCH" } });
  }
};

export const fetchImportStatusForURL = onceAtATime(_fetchImportStatusForURL);
