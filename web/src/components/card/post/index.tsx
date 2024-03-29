import { FC, forwardRef } from "react";
import { Stack } from "src/components/stack";
import { StyleProps } from "src/utils/props/style";
import { Text } from "src/components/text";
import { LoneModel } from "src/utils/models/lone-model";
import { CompactPost } from "src/models/post";
import { CompactTag } from "src/models/tag";
import { AccountType, CompactAccount } from "src/models/account";
import "./style.css";
import { Link } from "src/components/link";
import { getAccountName } from "src/utils/models/account-name";
import { Tag } from "src/components/tag";
import { getPostUrl } from "src/utils/urls/post-url";

export interface PostCardProps extends StyleProps {
  post: LoneModel<CompactPost> & {
    tags: Array<LoneModel<CompactTag>>;
    poster: LoneModel<CompactAccount> & AccountType;
  };
}

export const PostCard = forwardRef<HTMLAnchorElement, PostCardProps>(({ margin, post }, ref) => {
  return (
    <Link
      className="post-card"
      variant="v4"
      to={getPostUrl(post, post.poster)}
      margin={margin}
      ref={ref}
    >
      <div style={{ viewTransitionName: `post-title-${post?.id}` }}>
        <Text variant="v3" margin="0 0 1">
          {post.title}
        </Text>
      </div>
      <div style={{ viewTransitionName: `post-description-${post.id}`, flex: 1 }}>
        <Text variant="v5">{post.short_description}</Text>
      </div>
      {post.tags.length > 0 && (
        <div style={{ viewTransitionName: `post-tags-${post.id}` }}>
          <Stack orientation="horizontal" margin="1 0 0">
            {post.tags.slice(0, 3).map((tag) => (
              <Tag variant="v5" key={tag.id}>
                {tag.name}
              </Tag>
            ))}
            {post.tags.length > 3 && (
              <Tag variant="v5" key={post.tags[3].id}>
                +{post.tags.length - 3}
              </Tag>
            )}
          </Stack>
        </div>
      )}
      <Stack orientation="horizontal" margin="1 0 0" flex="0">
        <div style={{ viewTransitionName: `post-poster-${post.id}` }}>
          <Text variant="v5">{getAccountName(post.poster)}</Text>
        </div>
      </Stack>
    </Link>
  );
});
