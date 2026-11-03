import React from "react";
import { Icon, type IconName } from "../icons.js";

export interface ChipProps {
  label: string;
  active?: boolean;
  href?: string;
  dataGo?: string;
  onClick?: () => void;
  className?: string;
  style?: React.CSSProperties;
}

export const Chip: React.FC<ChipProps> = ({
  label,
  active,
  href,
  dataGo,
  onClick,
  className = "",
  style,
}) => {
  const classes = `chip ${active ? "on" : ""} ${className}`;
  const finalHref = href || (dataGo ? `#${dataGo}` : undefined);

  if (finalHref) {
    return (
      <a
        className={classes}
        href={finalHref}
        data-go={dataGo}
        style={{ textDecoration: "none", ...style }}
        onClick={onClick}
      >
        {label}
      </a>
    );
  }

  return (
    <button className={classes} type="button" style={style} onClick={onClick}>
      {label}
    </button>
  );
};

export interface PillProps {
  label: string;
  variant?: "ok" | "warn" | "bad" | "ai" | string;
  className?: string;
  style?: React.CSSProperties;
}

export const Pill: React.FC<PillProps> = ({ label, variant, className = "", style }) => {
  return (
    <span className={`pill ${variant || ""} ${className}`} style={style}>
      {label}
    </span>
  );
};

export interface FieldProps {
  label: string;
  children: React.ReactNode;
  className?: string;
  style?: React.CSSProperties;
}

export const Field: React.FC<FieldProps> = ({ label, children, className = "", style }) => {
  return (
    <label className={`fld ${className}`} style={style}>
      <span>{label}</span>
      {children}
    </label>
  );
};

export interface InputProps {
  value?: string;
  defaultValue?: string;
  placeholder?: string;
  onChange?: (e: React.ChangeEvent<HTMLInputElement>) => void;
  className?: string;
  style?: React.CSSProperties;
  disabled?: boolean;
}

export const Input: React.FC<InputProps> = ({
  value,
  defaultValue,
  placeholder,
  onChange,
  className = "",
  style,
  disabled,
}) => {
  return (
    <input
      className={`inp ${className}`}
      value={value}
      defaultValue={defaultValue}
      placeholder={placeholder}
      onChange={onChange}
      style={style}
      disabled={disabled}
    />
  );
};

export interface SelectProps {
  options: readonly string[];
  selectedIndex?: number;
  value?: string;
  onChange?: (val: string, index: number) => void;
  className?: string;
  style?: React.CSSProperties;
  disabled?: boolean;
}

export const Select: React.FC<SelectProps> = ({
  options,
  selectedIndex = 0,
  value,
  onChange,
  className = "",
  style,
  disabled,
}) => {
  const currentVal = value ?? options[selectedIndex];

  return (
    <select
      className={`inp ${className}`}
      value={currentVal}
      onChange={(e) => onChange?.(e.target.value, options.indexOf(e.target.value))}
      style={style}
      disabled={disabled}
    >
      {options.map((opt) => (
        <option key={opt} value={opt}>
          {opt}
        </option>
      ))}
    </select>
  );
};

export interface SectionProps {
  title: string;
  children: React.ReactNode;
  className?: string;
  style?: React.CSSProperties;
}

export const Section: React.FC<SectionProps> = ({ title, children, className = "", style }) => {
  return (
    <section className={`sec ${className}`} style={style}>
      <h4>{title}</h4>
      {children}
    </section>
  );
};

export interface TileProps {
  icon?: IconName | string;
  title: string;
  subtitle: string;
  href?: string;
  dataGo?: string;
  active?: boolean;
  className?: string;
  style?: React.CSSProperties;
  onClick?: () => void;
  children?: React.ReactNode;
}

export const Tile: React.FC<TileProps> = ({
  icon,
  title,
  subtitle,
  href,
  dataGo,
  active,
  className = "",
  style,
  onClick,
  children,
}) => {
  const classes = `tile ${active ? "on" : ""} ${className}`;
  const finalHref = href || (dataGo ? `#${dataGo}` : undefined);

  if (finalHref) {
    return (
      <a className={classes} href={finalHref} data-go={dataGo} style={style} onClick={onClick}>
        {children}
        {icon && <Icon name={icon} />}
        <b>{title}</b>
        <span>{subtitle}</span>
      </a>
    );
  }

  return (
    <button className={classes} type="button" style={style} onClick={onClick}>
      {children}
      {icon && <Icon name={icon} />}
      <b>{title}</b>
      <span>{subtitle}</span>
    </button>
  );
};

export interface CalloutProps {
  icon?: IconName | string;
  variant?: "warn" | "bad" | "ac" | "info" | string;
  children: React.ReactNode;
  className?: string;
  style?: React.CSSProperties;
}

export const Callout: React.FC<CalloutProps> = ({
  icon = "info",
  variant,
  children,
  className = "",
  style,
}) => {
  return (
    <div className={`callout ${variant || ""} ${className}`} style={style}>
      <Icon name={icon} />
      <div>{children}</div>
    </div>
  );
};

export interface ProgressBarProps {
  percent: number;
  running?: boolean;
  className?: string;
  style?: React.CSSProperties;
}

export const ProgressBar: React.FC<ProgressBarProps> = ({
  percent,
  running,
  className = "",
  style,
}) => {
  const clamped = Math.min(100, Math.max(0, percent));
  return (
    <div
      className={`prog ${running ? "run" : ""} ${className}`}
      style={{ "--v": `${clamped}%`, ...style } as React.CSSProperties}
      role="progressbar"
      aria-valuenow={clamped}
      aria-valuemin={0}
      aria-valuemax={100}
    >
      <i />
    </div>
  );
};
