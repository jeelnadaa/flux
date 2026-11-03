import React from "react";
import { Icon, type IconName } from "../icons.js";

export interface ButtonProps {
  label?: string;
  variant?: "primary" | "secondary" | "ghost" | "magic" | "danger" | string;
  large?: boolean;
  smallLabel?: boolean;
  icon?: IconName | string;
  iconOnly?: boolean;
  href?: string;
  dataGo?: string;
  ariaLabel?: string;
  disabled?: boolean;
  className?: string;
  style?: React.CSSProperties;
  onClick?: (event: React.MouseEvent<HTMLButtonElement | HTMLAnchorElement>) => void;
  onPointerDown?: (event: React.PointerEvent<HTMLButtonElement | HTMLAnchorElement>) => void;
  hold?: boolean;
  dataPlay?: boolean;
  dataToggle?: boolean;
  title?: string;
  children?: React.ReactNode;
  type?: "button" | "submit" | "reset";
  "aria-pressed"?: boolean | "true" | "false";
  "aria-haspopup"?: boolean | "true" | "false" | "dialog" | "menu";
  "aria-expanded"?: boolean | "true" | "false";
}

export const Button: React.FC<ButtonProps> = ({
  label,
  variant,
  large,
  smallLabel,
  icon,
  iconOnly,
  href,
  dataGo,
  ariaLabel,
  disabled,
  className = "",
  style,
  onClick,
  onPointerDown,
  hold,
  dataPlay,
  dataToggle,
  title,
  children,
  type = "button",
  "aria-pressed": ariaPressed,
  "aria-haspopup": ariaHasPopup,
  "aria-expanded": ariaExpanded,
}) => {
  const isIconOnly = iconOnly || (!label && !children);
  const variantClass =
    variant === "primary"
      ? "p"
      : variant === "ghost"
        ? "g"
        : variant === "danger"
          ? "dng"
          : variant === "magic"
            ? "magic"
            : variant || "";

  const classes = [
    "btn",
    variantClass,
    large ? "lg" : "",
    smallLabel ? "lbl-s" : "",
    isIconOnly ? "ic" : "",
    hold ? "hold" : "",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  const content = (
    <>
      {icon && <Icon name={icon} />}
      {label && <span>{label}</span>}
      {children}
    </>
  );

  const finalHref = href || (dataGo ? `#${dataGo}` : undefined);

  if (finalHref) {
    return (
      <a
        className={classes}
        href={finalHref}
        data-go={dataGo}
        aria-label={ariaLabel || (!label && !children ? title : undefined)}
        style={style}
        title={title}
        onClick={onClick}
      >
        {content}
      </a>
    );
  }

  return (
    <button
      className={classes}
      type={type}
      disabled={disabled}
      aria-label={ariaLabel || (!label && !children ? title : undefined)}
      aria-pressed={ariaPressed}
      aria-haspopup={ariaHasPopup}
      aria-expanded={ariaExpanded}
      data-go={dataGo}
      data-play={dataPlay ? "" : undefined}
      data-toggle={dataToggle ? "" : undefined}
      data-hold={hold ? "" : undefined}
      style={style}
      title={title}
      onClick={onClick}
      onPointerDown={onPointerDown}
    >
      {content}
    </button>
  );
};
