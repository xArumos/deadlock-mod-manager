import { describe, expect, it } from "bun:test";
import { create } from "zustand";
import { createGameSlice, type GameState } from "./game";

const createStore = () =>
  create<GameState>()((set, get, store) =>
    createGameSlice(
      set as Parameters<typeof createGameSlice>[0],
      get as Parameters<typeof createGameSlice>[1],
      store as Parameters<typeof createGameSlice>[2],
    ),
  );

describe("download location", () => {
  it("defaults to the automatic location", () => {
    const state = createStore().getState();

    expect(state.useCustomDownloadPath).toBe(false);
    expect(state.downloadPath).toBe("");
  });

  it("keeps the chosen folder while a custom folder is in use", () => {
    const store = createStore();
    store.getState().setDownloadPath("D:\\Games");
    store.getState().setUseCustomDownloadPath(true);

    expect(store.getState().useCustomDownloadPath).toBe(true);
    expect(store.getState().downloadPath).toBe("D:\\Games");
  });

  it("forgets the chosen folder when switching back to automatic", () => {
    const store = createStore();
    store.getState().setDownloadPath("D:\\Games");
    store.getState().setUseCustomDownloadPath(true);
    store.getState().setUseCustomDownloadPath(false);

    expect(store.getState().useCustomDownloadPath).toBe(false);
    expect(store.getState().downloadPath).toBe("");
  });
});
