import { describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import {
  useAchievements,
  useApplyReidentify,
  useListGameMediaFiles,
  useSearchForReidentify,
  useSelectBoxartFile,
  type GameAchievementsProgress,
  type ReidentifyCandidate,
} from "./use-game-actions";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

function wrapper({ children }: { children: ReactNode }) {
  const queryClient = new QueryClient();
  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
}

describe("useSearchForReidentify", () => {
  it("invokes search_for_reidentify with the free-text query and returns the candidates", async () => {
    const candidates: ReidentifyCandidate[] = [{ steamgriddb_id: 99, title: "Chrono Trigger", boxart_url: "https://example.com/a.png" }];
    invokeMock.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "search_for_reidentify" && args?.query === "chrono trigger") return Promise.resolve(candidates);
      throw new Error(`unexpected invoke: ${cmd} ${JSON.stringify(args)}`);
    });

    const { result } = renderHook(() => useSearchForReidentify(), { wrapper });

    await act(async () => {
      const matches = await result.current.mutateAsync("chrono trigger");
      expect(matches).toEqual(candidates);
    });
  });
});

describe("useApplyReidentify", () => {
  it("invokes apply_reidentify with the expected args and invalidates games/library on success", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "apply_reidentify") return Promise.resolve(undefined);
      throw new Error(`unexpected invoke: ${cmd}`);
    });

    const queryClient = new QueryClient();
    const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");
    const localWrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );

    const { result } = renderHook(() => useApplyReidentify(), { wrapper: localWrapper });

    await act(async () => {
      await result.current.mutateAsync({ gameId: "g1", steamgriddbId: 99, title: "Chrono Trigger" });
    });

    expect(invokeMock).toHaveBeenCalledWith("apply_reidentify", { gameId: "g1", steamgriddbId: 99, title: "Chrono Trigger" });
    await waitFor(() => {
      expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ["games"] });
      expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ["library"] });
    });
  });
});

describe("useListGameMediaFiles", () => {
  it("invokes list_game_media_files with the game id and returns the filenames", async () => {
    invokeMock.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "list_game_media_files" && args?.gameId === "g1") return Promise.resolve(["boxart-1.png", "cover.jpg"]);
      throw new Error(`unexpected invoke: ${cmd} ${JSON.stringify(args)}`);
    });

    const { result } = renderHook(() => useListGameMediaFiles("g1"), { wrapper });

    await waitFor(() => expect(result.current.data).toEqual(["boxart-1.png", "cover.jpg"]));
  });
});

describe("useSelectBoxartFile", () => {
  it("invokes select_boxart_file with the expected args and invalidates games/library on success", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "select_boxart_file") return Promise.resolve(undefined);
      throw new Error(`unexpected invoke: ${cmd}`);
    });

    const queryClient = new QueryClient();
    const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");
    const localWrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );

    const { result } = renderHook(() => useSelectBoxartFile(), { wrapper: localWrapper });

    await act(async () => {
      await result.current.mutateAsync({ gameId: "g1", filename: "cover.jpg" });
    });

    expect(invokeMock).toHaveBeenCalledWith("select_boxart_file", { gameId: "g1", filename: "cover.jpg" });
    await waitFor(() => {
      expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ["games"] });
      expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ["library"] });
    });
  });
});

describe("useAchievements", () => {
  it("returns null when the game isn't RA-matched (or no profile is linked)", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "get_achievements") return Promise.resolve(null);
      throw new Error(`unexpected invoke: ${cmd}`);
    });

    const { result } = renderHook(() => useAchievements("g1"), { wrapper });

    await waitFor(() => expect(result.current.data).toBeNull());
  });

  it("returns the achievement progress from get_achievements", async () => {
    const progress: GameAchievementsProgress = {
      game_id: 99,
      title: "Chrono Trigger",
      console_name: "SNES",
      num_achievements: 1,
      num_awarded_to_user: 1,
      user_completion: "100.00%",
      highest_award_kind: "mastered",
      achievements: [{ id: 1, title: "Time's Up", description: "Beat the game", points: 10, badge_url: "https://i.retroachievements.org/Badge/12345.png", unlocked: true }],
    };
    invokeMock.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "get_achievements" && args?.gameId === "g1") return Promise.resolve(progress);
      throw new Error(`unexpected invoke: ${cmd}`);
    });

    const { result } = renderHook(() => useAchievements("g1"), { wrapper });

    await waitFor(() => expect(result.current.data).toEqual(progress));
  });
});
