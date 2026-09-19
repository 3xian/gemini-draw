import { useEffect, useState, type CSSProperties } from "react";
import { api } from "../lib/api";

const cache = new Map<string, string>();
const inflight = new Map<string, Promise<string>>();

function key(path: string, maxSide: number | null) {
  return `${maxSide ?? "full"}:${path}`;
}

async function load(path: string, maxSide: number | null): Promise<string> {
  const k = key(path, maxSide);
  const hit = cache.get(k);
  if (hit) return hit;
  let p = inflight.get(k);
  if (!p) {
    p = api
      .readImage(path, maxSide ?? undefined)
      .then((b64) => {
        cache.set(k, b64);
        inflight.delete(k);
        return b64;
      })
      .catch((e) => {
        inflight.delete(k);
        throw e;
      });
    inflight.set(k, p);
  }
  return p;
}

interface Props {
  path: string;
  maxSide?: number | null;
  className?: string;
  style?: CSSProperties;
  onClick?: () => void;
  title?: string;
}

export function Thumb({ path, maxSide = 480, className, style, onClick, title }: Props) {
  const [src, setSrc] = useState<string | null>(() => cache.get(key(path, maxSide)) ?? null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    if (!path) return;
    let alive = true;
    setSrc(cache.get(key(path, maxSide)) ?? null);
    setFailed(false);
    load(path, maxSide)
      .then((b64) => {
        if (alive) setSrc(b64);
      })
      .catch(() => {
        if (alive) setFailed(true);
      });
    return () => {
      alive = false;
    };
  }, [path, maxSide]);

  if (failed) {
    return (
      <div className={`${className ?? ""} skeleton`} style={style} title={title ?? "加载失败"} />
    );
  }
  if (!src) {
    return <div className={`${className ?? ""} skeleton`} style={style} />;
  }
  return (
    <img
      className={className}
      style={style}
      src={`data:image/png;base64,${src}`}
      onClick={onClick}
      title={title}
      draggable={false}
      alt=""
    />
  );
}
