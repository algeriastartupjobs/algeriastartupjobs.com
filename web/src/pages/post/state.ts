import { createSlice } from "@reduxjs/toolkit";
import { PostCardProps } from "src/components/card/post";
import { Account, AccountType, CompactAccount } from "src/models/account";
import { CompactPost, Post } from "src/models/post";
import { CompactTag } from "src/models/tag";
import { LOADABLE } from "src/utils/loadable";
import { LoneModel } from "src/utils/models/lone-model";
import { overWriterReducerFactory, setterReducerFactory } from "src/utils/state/reducer";
import { UnionWithOptionalDiscriminatedProps } from "src/utils/types/union";

export interface PostPageState {
  postId: string | null;
  post: LOADABLE<
    // @TODO-ZM: make CompactPost & Post work
    LoneModel<
      CompactPost &
        Partial<
          Post & {
            tags: Array<LoneModel<CompactTag>>;
            poster: LoneModel<UnionWithOptionalDiscriminatedProps<Account, CompactAccount>> &
              AccountType;
          }
        >
    >
  >;
  similarPosts: LOADABLE<Array<PostCardProps["post"]>>;
  deletion_status: "IDLE" | "DELETING" | "DELETED" | "ERROR";
}

export const postPage = createSlice({
  name: "postPage",
  initialState: {
    postId: null,
    post: null,
    similarPosts: null,
    deletion_status: "IDLE",
  } as PostPageState,
  reducers: {
    set: setterReducerFactory(),
    overwrite: overWriterReducerFactory(),
  },
});
