import Axios from "axios";
import { Account } from "src/models/account";
import { getState, getStateActions } from "src/state";
import { getConfig } from "src/utils/config/get-config";
import { initialStateForCreatePostPage } from "./state";
import { getBrowserRouter } from "src/components/router-provider";
import { CONFIRM_EMAIL_PAGE_URL } from "src/utils/urls/common";
import { initialStateForConfirmEmailPage } from "../confirm-email/state";

export const fetchAccountForCreatePostPage = async (): Promise<void> => {
  const { accountEntities, createPostPage } = getStateActions();
  const { poster_contact } = getState().createPostPage;

  if (!poster_contact) return;

  try {
    // @TODO-ZM: auto-generate types for API endpoints
    const { data } = await Axios.get<{
      account: Account;
    }>(getConfig().api.base_url + "/account/by_email?email=" + encodeURIComponent(poster_contact));

    createPostPage.set({ poster: data.account });

    // update cache:
    accountEntities.upsertMany([data.account]);
  } catch (error) {
    // @TODO-ZM: set it to null when status is 404
    createPostPage.set({ poster: "ERROR" });
    // @TODO-ZM: use Logger abstraction instead of console.log
    console.log("Error fetching posts for landing page", error);
    // Sentry.captureException(error, { tags: { type: "WEB_FETCH" } });
  }
};

export const createPost = async (): Promise<void> => {
  // @TODO-ZM: update post cache
  const { createPostPage, confirmEmailPage } = getStateActions();
  createPostPage.set({ creation_status: "CREATING" });

  // @TODO-ZM: Call backend, get post id, and render a link to the post
  await new Promise((resolve) => setTimeout(resolve, 1000));

  // createPostPage.set({ ...initialStateForCreatePostPage, creation_status: "ERROR" });
  createPostPage.set({ ...initialStateForCreatePostPage, creation_status: "CREATED" });
  confirmEmailPage.set(initialStateForConfirmEmailPage);

  getBrowserRouter().navigate(CONFIRM_EMAIL_PAGE_URL);
};