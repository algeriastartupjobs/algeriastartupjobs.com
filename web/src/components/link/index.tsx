import { FC, PropsWithChildren } from "react";
import "./style.css";
import { FontVariantProps, StyleProps } from "src/utils/props/style";
import { Link as RL, LinkProps as RLP } from "react-router-dom";

interface LinkProps
  extends PropsWithChildren,
    StyleProps,
    FontVariantProps,
    RLP {}

export const Link: FC<LinkProps> = ({
  children,
  variant,
  margin,
  ...props
}) => {
  const classes = [
    "link",
    `font-variant-${variant}`,
    margin && `margin-${margin}`,
  ].filter(Boolean);
  return (
    <RL className={classes.join(" ")} {...props}>
      {children}
    </RL>
  );
};