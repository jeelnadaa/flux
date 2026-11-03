import React from "react";

export interface ToggleProps {
  label: string;
  sub?: string;
  checked?: boolean;
  defaultChecked?: boolean;
  onChange?: (checked: boolean) => void;
  className?: string;
  style?: React.CSSProperties;
  ariaLabel?: string;
}

export const Toggle: React.FC<ToggleProps> = ({
  label,
  sub,
  checked,
  defaultChecked = false,
  onChange,
  className = "",
  style,
  ariaLabel,
}) => {
  const [internalChecked, setInternalChecked] = React.useState(checked ?? defaultChecked);
  const isChecked = checked !== undefined ? checked : internalChecked;

  const handleClick = () => {
    const next = !isChecked;
    setInternalChecked(next);
    onChange?.(next);
  };

  return (
    <button
      className={`tg ${className}`}
      type="button"
      aria-pressed={isChecked}
      aria-label={ariaLabel}
      onClick={handleClick}
      style={style}
    >
      <span>
        {label}
        {sub && <small>{sub}</small>}
      </span>
      <i />
    </button>
  );
};
