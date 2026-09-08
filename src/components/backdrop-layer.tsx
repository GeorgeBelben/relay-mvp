import { useEffect, useState } from "react";
import { useBackdropStore } from "@/lib/backdrop";
import { resolveMediaUrl, useMediaRoot } from "@/lib/media";
import { cn } from "@/lib/cn";

const FADE_MS = 700;

type Layer = { id: string; url: string };

// Mounted once at the app root (see __root.tsx), driven by useBackdropStore rather than local
// per-tile state -- CarouselGameTile focus just updates the store (via Home's heroGame effect)
// and this owns rendering it, same reasoning as LaunchOverlay/QuickMenu at this layer.
//
// A true crossfade (not just a fade-to-black-then-in) needs the outgoing image still visible
// while the incoming one fades over it, so this keeps a small stack of layers rather than a
// single <img> whose src gets swapped -- swapping src on one element snaps instantly, it doesn't
// transition. Each new backdrop is pushed on top starting at opacity-0, flipped to opacity-100 on
// the next frame (so the browser actually sees two different states to transition between), and
// once its own fade-in has finished, every layer beneath it is pruned -- it's now fully covering
// them, so dropping them causes no visible change.
export function BackdropLayer() {
  const game = useBackdropStore((state) => state.game);
  const mediaRoot = useMediaRoot();

  const backdropUrl = mediaRoot && game?.backdrop_path ? resolveMediaUrl(mediaRoot, game.backdrop_path) : null;

  const [layers, setLayers] = useState<Layer[]>([]);
  const [revealedIds, setRevealedIds] = useState<ReadonlySet<string>>(new Set());

  useEffect(() => {
    if (!backdropUrl) return;
    setLayers((prev) => (prev.at(-1)?.url === backdropUrl ? prev : [...prev, { id: `${backdropUrl}#${Date.now()}`, url: backdropUrl }]));
  }, [backdropUrl]);

  useEffect(() => {
    const top = layers.at(-1);
    if (!top || revealedIds.has(top.id)) return;

    const raf = requestAnimationFrame(() => {
      setRevealedIds((prev) => new Set(prev).add(top.id));
    });
    const prune = setTimeout(() => {
      setLayers((prev) => {
        const index = prev.findIndex((layer) => layer.id === top.id);
        return index <= 0 ? prev : prev.slice(index);
      });
    }, FADE_MS + 50);

    return () => {
      cancelAnimationFrame(raf);
      clearTimeout(prune);
    };
  }, [layers, revealedIds]);

  return (
    <div className="fixed inset-0 -z-10 overflow-hidden bg-[#111111]">
      {layers.map((layer) => (
        <img
          key={layer.id}
          src={layer.url}
          alt=""
          className={cn("absolute inset-0 h-full w-full object-cover transition-opacity", revealedIds.has(layer.id) ? "opacity-100" : "opacity-0")}
          style={{ transitionDuration: `${FADE_MS}ms` }}
        />
      ))}
      <div className="absolute inset-0 bg-gradient-to-t from-[#111111] via-[#111111]/50 to-[#111111]/10" />
    </div>
  );
}
