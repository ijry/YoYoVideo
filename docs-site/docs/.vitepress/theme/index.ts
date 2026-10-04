import DefaultTheme from "vitepress/theme";
import type { Theme } from "vitepress";
import "./style.css";

/**
 * The player chrome uses sky (#38bdf8) on near-black, so the docs brand colour is
 * taken from the app itself rather than invented separately.
 */
export default {
  extends: DefaultTheme,
} satisfies Theme;
