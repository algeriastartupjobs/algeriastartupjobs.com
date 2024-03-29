import { createSlice } from "@reduxjs/toolkit";
import { Account } from "src/models/account";
import { Tag } from "src/models/tag";
import { LOADABLE } from "src/utils/loadable";
import { overWriterReducerFactory, setterReducerFactory } from "src/utils/state/reducer";

export type CreatePostPageState = {
  title: string;
  poster_type: Account["type"];
  poster_name: string;
  poster_first_name: string;
  poster_last_name: string;
  poster_contact: string;
  poster: LOADABLE<Account>;
  creation_status: "IDLE" | "CREATING" | "CREATED" | "ERROR";
  suggested_tags: LOADABLE<Tag[]>;
  compact: boolean;
  post_description: string;
  tags: Tag[];
};

export const initialStateForCreatePostPage: CreatePostPageState = {
  title: "",
  poster_type: "Company",
  poster_name: "",
  poster_first_name: "",
  poster_last_name: "",
  poster_contact: "",
  poster: null,
  creation_status: "IDLE",
  compact: true,
  post_description: "",
  tags: [],
  suggested_tags: null,
};

export const createPostPage = createSlice({
  name: "createPostPage",
  initialState: initialStateForCreatePostPage as CreatePostPageState,
  reducers: {
    set: setterReducerFactory(),
    overwrite: overWriterReducerFactory(),
  },
});
