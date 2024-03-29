import { Account } from "src/models/account";
import { getState, getStateActions } from "src/state";
import { initialStateForCreatePostPage } from "./state";
import { getBrowserRouter } from "src/components/router-provider";
import { CONFIRM_EMAIL_PAGE_URL } from "src/utils/urls/common";
import { initialStateForConfirmEmailPage } from "../confirm-email/state";
import { Post } from "src/models/post";
import { CompactTag } from "src/models/tag";
import { onceAtATime } from "src/utils/concurrency/once-at-a-time";
import { fetch } from "src/utils/fetch/fetch";
import { PostPageState } from "../post/state";
import { getPostUrl } from "src/utils/urls/post-url";
import { viewTransitionSubscribeOnce } from "src/utils/animation/view-transition";
import * as Sentry from "@sentry/react";
import { AxiosError } from "axios";

export const fetchAccountForCreatePostPage = async (): Promise<void> => {
  const { poster_contact } = getState().createPostPage;
  if (!poster_contact) return;

  const { accountEntities, createPostPage } = getStateActions();

  try {
    // @TODO-ZM: auto-generate types for API endpoints
    const { data } = await fetch.get<{
      account: Account;
    }>("/accounts/by_email?email=" + encodeURIComponent(poster_contact));

    createPostPage.set({ poster: data.account });

    // update cache:
    accountEntities.upsertMany([data.account]);
  } catch (error) {
    // @TODO-ZM: set it to null when status is 404
    createPostPage.set({ poster: "ERROR" });
    // @TODO-ZM: use Logger abstraction instead of console.log
    Sentry.captureException(error, { tags: { type: "WEB_FETCH" } });
    console.log("Error fetching posts for landing page", error);
  }
};

const concurrentFetchTagsForCreatePostPage = async (): Promise<void> => {
  const { compact, title, post_description } = getState().createPostPage;
  const { tagEntities, createPostPage } = getStateActions();

  if (!post_description) createPostPage.set({ suggested_tags: [] });
  if (compact || !post_description) return;

  try {
    // @TODO-ZM: auto-generate types for API endpoints
    const { data } = await fetch.post<{
      tags: CompactTag[];
    }>("/tags/suggestions_for_post", {
      description: post_description,
      title,
    });

    if (!data.tags.length) return;

    createPostPage.set({ suggested_tags: data.tags });

    // update cache:
    tagEntities.upsertMany(data.tags);
  } catch (error) {
    if ((error as AxiosError)?.status === 429) return;

    createPostPage.set({ suggested_tags: "ERROR" });
    // @TODO-ZM: use Logger abstraction instead of console.log
    console.log("Error fetching suggested tags for create post page", error);
    Sentry.captureException(error, { tags: { type: "WEB_FETCH" } });
  }
};

export const fetchTagsForCreatePostPage = onceAtATime(concurrentFetchTagsForCreatePostPage, {
  runLastCall: true,
});

export const createPostViaEmail = async (): Promise<void> => {
  const { createPostPage, confirmEmailPage } = getStateActions();
  createPostPage.set({ creation_status: "CREATING" });

  try {
    const {
      title,
      poster_type,
      poster_name,
      poster_first_name,
      poster_last_name,
      poster_contact,
      post_description = "",
      tags = [],
    } = getState().createPostPage;

    const {
      data: { confirmation_id, post_id },
    } = await fetch.post<{
      post_id: number;
      poster_id: number;
      confirmation_id: string;
    }>("/posts/via_email", {
      poster: {
        email: poster_contact,
        slug: "",
        type: poster_type,
        ...(poster_type === "Company"
          ? {
              company_name: poster_name,
            }
          : {
              first_name: poster_first_name,
              last_name: poster_last_name,
            }),
      } as Omit<Account, "id">,
      post: {
        title,
        slug: "",
        short_description: "",
        description: post_description,
        poster_id: 0,
        tag_ids: tags.map((tag) => tag.id),
        is_published: false,
        published_at: "",
      } as Omit<Post, "id">,
    });

    createPostPage.set({ creation_status: "CREATED" });
    viewTransitionSubscribeOnce(() => {
      createPostPage.overwrite(initialStateForCreatePostPage);
    });
    confirmEmailPage.set({ ...initialStateForConfirmEmailPage, confirmation_id, post_id });
    getBrowserRouter().navigate(CONFIRM_EMAIL_PAGE_URL);
  } catch (error) {
    // @TODO-ZM: use Logger abstraction instead of console
    Sentry.captureException(error, { tags: { type: "WEB_FETCH" } });
    console.log("Error creating post", error);
    createPostPage.set({ creation_status: "ERROR" });
  }
};

export const createPost = async (): Promise<void> => {
  const { createPostPage, postPage, postEntities, tagEntities, accountEntities } =
    getStateActions();
  createPostPage.set({ creation_status: "CREATING" });

  try {
    const { title, post_description = "", tags = [] } = getState().createPostPage;

    const { data } = await fetch.post<{
      post: Post;
      poster: Account;
      tags: CompactTag[];
    }>("/posts", {
      post: {
        title,
        slug: "",
        short_description: "",
        description: post_description,
        poster_id: 0,
        tag_ids: tags.map((tag) => tag.id),
        is_published: false,
        published_at: "",
      } as Omit<Post, "id">,
    });

    createPostPage.set({ creation_status: "CREATED" });
    // @TODO-ZM: subscribe to next page navigation event instead of using setTimeout
    viewTransitionSubscribeOnce(() => {
      createPostPage.overwrite(initialStateForCreatePostPage);
    });

    const { tag_ids, poster_id, ...lonePost } = data.post;
    const post: PostPageState["post"] = {
      ...lonePost,
      tags: data.tags,
      poster: data.poster,
    };

    postPage.set({ post });

    const postUrl = getPostUrl(data.post, data.poster);
    getBrowserRouter().navigate(postUrl);

    // update cache:
    postEntities.upsertMany([data.post]);
    tagEntities.upsertMany(data.tags);
    accountEntities.upsertMany([data.poster]);
  } catch (error) {
    console.log("Error creating post", error);
    // @TODO-ZM: use Logger abstraction instead of console
    Sentry.captureException(error, { tags: { type: "WEB_FETCH" } });
    createPostPage.set({ creation_status: "ERROR" });
  }
};
