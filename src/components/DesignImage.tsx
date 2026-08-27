import React, { useEffect, useState } from "react";
import { readImageData } from "../lib";

const previewCache = new Map<string, string>();

interface DesignImageProps {
  previewPath?: string;
  title: string;
  format: string;
  className?: string;
  large?: boolean;
}

const formatColor = (format: string) => {
  const map: Record<string, string> = {
    PES: "#ef8d71",
    DST: "#48b0a5",
    JEF: "#9385e4",
    VP3: "#4f98d9",
    EXP: "#e6a23c",
    HUS: "#d977a8",
    XXX: "#50b57e",
    SEW: "#d97706",
    PCS: "#6366f1",
    PEC: "#ec4899",
  };
  return map[format.toUpperCase()] ?? "#8a9aad";
};

export const DesignImage: React.FC<DesignImageProps> = ({
  previewPath,
  title,
  format,
  className = "",
  large = false,
}) => {
  const [dataUri, setDataUri] = useState<string | null>(() => {
    return previewPath ? previewCache.get(previewPath) ?? null : null;
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(false);

  useEffect(() => {
    if (!previewPath) {
      setDataUri(null);
      return;
    }

    if (previewCache.has(previewPath)) {
      setDataUri(previewCache.get(previewPath)!);
      return;
    }

    let isMounted = true;
    setLoading(true);
    setError(false);

    readImageData(previewPath)
      .then((uri) => {
        if (isMounted) {
          previewCache.set(previewPath, uri);
          setDataUri(uri);
          setLoading(false);
        }
      })
      .catch(() => {
        if (isMounted) {
          setError(true);
          setLoading(false);
        }
      });

    return () => {
      isMounted = false;
    };
  }, [previewPath]);

  const initials = title
    .split(" ")
    .map((w) => w[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();

  const accent = formatColor(format);

  if (dataUri && !error) {
    return (
      <div className={`design-image-container ${large ? "large" : ""} ${className}`}>
        <img
          src={dataUri}
          alt={title}
          className="rendered-preview-img"
          loading="lazy"
        />
        <span className="format-overlay-pill" style={{ backgroundColor: accent }}>
          {format}
        </span>
      </div>
    );
  }

  return (
    <div
      className={`design-image-fallback ${large ? "large" : ""} ${className}`}
      style={{ "--accent": accent } as React.CSSProperties}
    >
      <span className="hoop-guide" />
      <span className="stitch-pattern stitch-1" />
      <span className="stitch-pattern stitch-2" />
      <b className="initials-badge">{initials || "??"}</b>
      <em className="format-indicator">{format}</em>
    </div>
  );
};
