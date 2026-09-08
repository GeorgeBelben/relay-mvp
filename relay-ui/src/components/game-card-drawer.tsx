import { useEffect, useRef, useState } from "react";
import { RiImageEditLine, RiLoader4Line } from "@remixicon/react";
import { toast } from "sonner";
import { Drawer } from "./drawer";
import { List, ListRow } from "./list";
import { SearchField } from "./search-field";
import { AchievementsView } from "./achievements-view";
import { FocusContext, useBackHandler, useFocusable } from "@/lib/focus";
import { playSound } from "@/lib/sound";
import { cn } from "@/lib/cn";
import {
  useApplyReidentify,
  useListGameMediaFiles,
  useSearchForReidentify,
  useSelectBoxartFile,
  type ReidentifyCandidate,
} from "@/hooks/use-game-actions";
import { useGame } from "@/hooks/use-games";
import { resolveMediaUrl, useMediaRoot } from "@/lib/media";
import type { LibraryGame } from "@/hooks/use-library";

type GameCardDrawerProps = {
  game: LibraryGame;
  open: boolean;
  onClose: () => void;
};

type View = "menu" | "artwork" | "reidentify" | "achievements";

// Context menu for a single game card (see carousel-game-tile.tsx / game-tile.tsx), opened by
// the "menu" action (Y/Triangle/X, or M on keyboard -- see lib/input/keyboard.ts). Four views
// inside one drawer: the action list, a browser over this game's own media folder ("Artwork"),
// a free-text SteamGridDB search for fixing a wrong match ("Reidentify"), and this game's
// RetroAchievements list. Back from any of them returns to the menu instead of closing the
// drawer -- each registers its own back-handler, layered on top of Drawer's own (LIFO, see
// lib/focus/backStack.ts), only while that sub-view is actually showing.
export function GameCardDrawer({ game, open, onClose }: GameCardDrawerProps) {
  const [view, setView] = useState<View>("menu");

  // Explicit here for the same reason drawer.tsx/modal.tsx play their own "back" on a real
  // open->close transition: Base UI's Dialog swallows the physical Escape keydown outright while
  // it's open (see drawer.tsx's own comment), so the generic input-stream sound never fires for a
  // keyboard back-press in that state. Drawer's own sound only covers *its* open/close transition,
  // not this sub-view swap, which doesn't touch `open` at all -- without this, keyboard Escape
  // from a sub-view back to the menu was silent (gamepad B still worked, since it never goes
  // through Base UI's Dialog at all). Already debounced against itself in soundManager.ts, so this
  // collapses cleanly with whatever the generic stream already played for a gamepad-driven press
  // instead of doubling up.
  const backToMenu = () => {
    playSound("back");
    setView("menu");
  };

  const closeAndReset = () => {
    onClose();
    // Deliberately not reset immediately -- reset after the close transition, so the drawer
    // doesn't visibly flash back to the menu view while it's still animating shut.
    setTimeout(() => setView("menu"), 200);
  };

  return (
    // Every sub-view grows to 2/3 width -- media tiles and achievement descriptions are too
    // cramped at the menu's narrow max-w-sm. Menu view stays narrow since it's just a few rows.
    <Drawer
      open={open}
      onClose={closeAndReset}
      focusKey={`GAME_DRAWER_${game.id}`}
      className={view !== "menu" ? "max-w-[66vw]" : undefined}
    >
      {view === "menu" && (
        <GameCardMenu
          game={game}
          onArtwork={() => setView("artwork")}
          onReidentify={() => setView("reidentify")}
          onAchievements={() => setView("achievements")}
        />
      )}
      {view === "artwork" && (
        <ArtworkPicker game={game} onBack={backToMenu} onApplied={closeAndReset} />
      )}
      {view === "reidentify" && (
        <ReidentifyView game={game} onBack={backToMenu} onApplied={closeAndReset} />
      )}
      {view === "achievements" && <AchievementsView game={game} onBack={backToMenu} />}
    </Drawer>
  );
}

// game.beaten comes from RetroAchievements' own award ladder, not a self-reported flag -- there's
// nothing left for this menu to let the player set directly; opening Achievements is what
// actually refreshes it (see get_achievements).
function GameCardMenu({
  game,
  onArtwork,
  onReidentify,
  onAchievements,
}: {
  game: LibraryGame;
  onArtwork: () => void;
  onReidentify: () => void;
  onAchievements: () => void;
}) {
  return (
    <>
      <h2 className="truncate px-4 pb-2 text-base font-semibold">{game.title}</h2>
      <List>
        <ListRow label="Artwork" onSelect={onArtwork} />
        <ListRow label="Reidentify" onSelect={onReidentify} />
        <ListRow label="Achievements" onSelect={onAchievements} />
      </List>
    </>
  );
}

// Pure folder browser over this game's media directory -- no network involved. Every file there
// (dropped in by the user directly, or downloaded by an earlier enrich/reidentify) is a
// selectable candidate; picking one just repoints the active box art at it.
function ArtworkPicker({
  game,
  onBack,
  onApplied,
}: {
  game: LibraryGame;
  onBack: () => void;
  onApplied: () => void;
}) {
  useBackHandler(onBack);

  const files = useListGameMediaFiles(game.id);
  const mediaRoot = useMediaRoot();
  const selectBoxart = useSelectBoxartFile();

  // Same focus-landing reasoning as ReidentifyView/AchievementsView below: this view's tiles
  // don't exist until the file list resolves, so focusSelf() has nothing to descend into before
  // then.
  const { ref, focusKey, focusSelf } = useFocusable({
    trackChildren: true,
    saveLastFocusedChild: true,
  });
  useEffect(() => {
    if (files.data) focusSelf();
  }, [files.data, focusSelf]);

  const folderPath = mediaRoot ? `${mediaRoot}/${game.system_id}/${game.id}` : undefined;

  return (
    <FocusContext.Provider value={focusKey}>
      <div ref={ref}>
        <h2 className="truncate px-4 pb-2 text-base font-semibold">Artwork</h2>

        {files.isPending && (
          <p className="flex items-center gap-2 px-4 py-6 text-sm text-muted-foreground">
            <RiLoader4Line className="h-4 w-4 animate-spin" aria-hidden="true" />
            Loading…
          </p>
        )}

        {files.isError && (
          <p className="px-4 py-6 text-sm text-destructive">Couldn't read this game's media folder.</p>
        )}

        {files.data && files.data.length === 0 && (
          <p className="px-4 py-6 text-sm text-muted-foreground">
            No artwork yet. Drop image files into{" "}
            {folderPath ? <span className="break-all text-foreground">{folderPath}</span> : "this game's media folder"},
            or use Reidentify to fetch official art.
          </p>
        )}

        {files.data && files.data.length > 0 && mediaRoot && (
          <div className="grid grid-cols-4 gap-3 px-4 py-2 sm:grid-cols-6">
            {files.data.map((filename) => (
              <MediaFileTile
                key={filename}
                filename={filename}
                url={resolveMediaUrl(mediaRoot, `${game.system_id}/${game.id}/${filename}`)}
                applying={selectBoxart.isPending && selectBoxart.variables?.filename === filename}
                disabled={selectBoxart.isPending}
                onSelect={() =>
                  selectBoxart.mutate(
                    { gameId: game.id, filename },
                    {
                      onSuccess: onApplied,
                      onError: () => toast("Couldn't select that image"),
                    },
                  )
                }
              />
            ))}
          </div>
        )}
      </div>
    </FocusContext.Provider>
  );
}

function MediaFileTile({
  filename,
  url,
  applying,
  disabled,
  onSelect,
}: {
  filename: string;
  url: string;
  applying: boolean;
  disabled: boolean;
  onSelect: () => void;
}) {
  const { ref, focused } = useFocusable({ focusable: !disabled, onEnterPress: onSelect });

  return (
    <div
      ref={ref}
      className={cn(
        "relative flex aspect-2/3 flex-col justify-end overflow-hidden rounded-lg bg-gray-800 p-2 transition-bounce",
        focused && "scale-105 ring-4 ring-inset ring-amber-500",
        disabled && !applying && "opacity-40",
      )}
    >
      <img src={url} alt="" className="absolute inset-0 h-full w-full object-cover" />
      {applying && (
        <div className="absolute inset-0 flex items-center justify-center bg-black/50">
          <RiLoader4Line className="h-6 w-6 animate-spin text-white" aria-hidden="true" />
        </div>
      )}
      <span className="relative line-clamp-2 text-xs font-medium text-white drop-shadow">{filename}</span>
    </div>
  );
}

// Free-text SteamGridDB search -- for when the automatic match picked the wrong game (or none at
// all). Prefilled from the game's filename-derived scanned_title (searching by its current,
// possibly-wrong title would just re-fetch the same bad candidates), but freely editable. Applying
// a result re-points the game's identity AND seeds its media folder with a new file -- see
// apply_reidentify -- without touching anything already in that folder.
function ReidentifyView({
  game,
  onBack,
  onApplied,
}: {
  game: LibraryGame;
  onBack: () => void;
  onApplied: () => void;
}) {
  useBackHandler(onBack);

  const gameDetails = useGame(game.id);
  const [query, setQuery] = useState("");
  const prefilled = useRef(false);
  useEffect(() => {
    if (prefilled.current || !gameDetails.data) return;
    prefilled.current = true;
    setQuery(gameDetails.data.scanned_title ?? gameDetails.data.title);
  }, [gameDetails.data]);

  const search = useSearchForReidentify();
  useEffect(() => {
    if (!query) return;
    const timeout = setTimeout(() => search.mutate(query), 300);
    return () => clearTimeout(timeout);
    // Only re-fires when the query text itself changes -- `search` is a fresh mutation object
    // every render, and including it here would refire on every keystroke's own state update.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query]);

  // Same "focus once results exist" reasoning as the old BoxArtPicker: the result tiles don't
  // exist on first render, so focusSelf() would have nothing to land on before the search
  // resolves.
  const { ref, focusKey, focusSelf } = useFocusable({
    trackChildren: true,
    saveLastFocusedChild: true,
  });
  useEffect(() => {
    if (search.data) focusSelf();
  }, [search.data, focusSelf]);

  const applyReidentify = useApplyReidentify();

  return (
    <FocusContext.Provider value={focusKey}>
      <div ref={ref}>
        <h2 className="truncate px-4 pb-2 text-base font-semibold">Reidentify</h2>
        <div className="px-4 pb-2">
          <SearchField value={query} onChange={setQuery} placeholder="Search SteamGridDB…" />
        </div>

        {search.isPending && (
          <p className="flex items-center gap-2 px-4 py-6 text-sm text-muted-foreground">
            <RiLoader4Line className="h-4 w-4 animate-spin" aria-hidden="true" />
            Searching SteamGridDB…
          </p>
        )}

        {search.isError && (
          <p className="px-4 py-6 text-sm text-destructive">Couldn't search SteamGridDB.</p>
        )}

        {search.data && search.data.length === 0 && (
          <p className="px-4 py-6 text-sm text-muted-foreground">No matches found for "{query}".</p>
        )}

        {search.data && search.data.length > 0 && (
          <div className="grid grid-cols-4 gap-3 px-4 py-2 sm:grid-cols-6">
            {search.data.map((candidate) => (
              <ReidentifyTile
                key={candidate.steamgriddb_id}
                candidate={candidate}
                applying={
                  applyReidentify.isPending &&
                  applyReidentify.variables?.steamgriddbId === candidate.steamgriddb_id
                }
                disabled={applyReidentify.isPending}
                onSelect={() =>
                  applyReidentify.mutate(
                    {
                      gameId: game.id,
                      steamgriddbId: candidate.steamgriddb_id,
                      title: candidate.title,
                    },
                    {
                      onSuccess: onApplied,
                      onError: () => toast("Couldn't apply that match"),
                    },
                  )
                }
              />
            ))}
          </div>
        )}
      </div>
    </FocusContext.Provider>
  );
}

function ReidentifyTile({
  candidate,
  applying,
  disabled,
  onSelect,
}: {
  candidate: ReidentifyCandidate;
  applying: boolean;
  disabled: boolean;
  onSelect: () => void;
}) {
  const { ref, focused } = useFocusable({ focusable: !disabled, onEnterPress: onSelect });

  return (
    <div
      ref={ref}
      className={cn(
        "relative flex aspect-2/3 flex-col justify-end overflow-hidden rounded-lg bg-gray-800 p-2 transition-bounce",
        focused && "scale-105 ring-4 ring-inset ring-amber-500",
        disabled && !applying && "opacity-40",
      )}
    >
      {candidate.boxart_url && (
        <img
          src={candidate.boxart_url}
          alt=""
          className="absolute inset-0 h-full w-full object-cover"
        />
      )}
      {applying && (
        <div className="absolute inset-0 flex items-center justify-center bg-black/50">
          <RiLoader4Line className="h-6 w-6 animate-spin text-white" aria-hidden="true" />
        </div>
      )}
      {!candidate.boxart_url && !applying && (
        <RiImageEditLine
          className="absolute inset-0 m-auto h-6 w-6 text-muted-foreground"
          aria-hidden="true"
        />
      )}
      <span className="relative line-clamp-2 text-xs font-medium text-white drop-shadow">
        {candidate.title}
      </span>
    </div>
  );
}
