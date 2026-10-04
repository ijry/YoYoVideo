import { defineConfig, type DefaultTheme } from "vitepress";

const REPO = "https://github.com/ijry/YoYoVideo";
const RELEASES = `${REPO}/releases/latest`;
// Trailing slash is required: generateSitemap feeds SitemapStream relative paths,
// so a hostname without it silently drops the /YoYoVideo/ segment from every <loc>.
const HOSTNAME = "https://ijry.github.io/YoYoVideo/";

/* ----------------------------- 简体中文 ----------------------------- */

const zhNav: DefaultTheme.NavItem[] = [
  { text: "指南", link: "/guide/introduction", activeMatch: "^/guide/" },
  { text: "开发", link: "/dev/build", activeMatch: "^/dev/" },
  { text: "FAQ", link: "/guide/faq" },
  { text: "下载", link: RELEASES },
];

const zhSidebar: DefaultTheme.Sidebar = {
  "/guide/": [
    {
      text: "开始使用",
      collapsed: false,
      items: [
        { text: "YoYoVideo 是什么", link: "/guide/introduction" },
        { text: "安装与下载", link: "/guide/installation" },
        { text: "功能一览", link: "/guide/features" },
        { text: "常见问题", link: "/guide/faq" },
      ],
    },
  ],
  "/dev/": [
    {
      text: "开发",
      collapsed: false,
      items: [
        { text: "本地构建", link: "/dev/build" },
        { text: "架构总览", link: "/dev/architecture" },
        { text: "发布流程", link: "/dev/release" },
      ],
    },
  ],
};

/* ------------------------------ English ------------------------------ */

const enNav: DefaultTheme.NavItem[] = [
  { text: "Guide", link: "/en/guide/introduction", activeMatch: "^/en/guide/" },
  { text: "Develop", link: "/en/dev/build", activeMatch: "^/en/dev/" },
  { text: "Download", link: RELEASES },
];

// Sidebar keys must carry the /en/ prefix. getSidebar matches on relativePath,
// so a "/guide/" key would never match en/guide/*.md and English pages would
// silently render without a sidebar.
const enSidebar: DefaultTheme.Sidebar = {
  "/en/guide/": [
    {
      text: "Getting Started",
      collapsed: false,
      items: [
        { text: "What is YoYoVideo", link: "/en/guide/introduction" },
        { text: "Installation", link: "/en/guide/installation" },
        { text: "Features", link: "/en/guide/features" },
      ],
    },
  ],
  "/en/dev/": [
    {
      text: "Develop",
      collapsed: false,
      items: [
        { text: "Building locally", link: "/en/dev/build" },
        { text: "Release process", link: "/en/dev/release" },
      ],
    },
  ],
};

export default defineConfig({
  title: "YoYoVideo",
  description:
    "YoYoVideo 是一款用 Rust + Slint + libmpv 打造的全格式本地视频播放器，支持多画面批量播放、字幕与音轨切换、画面滤镜与 A-B 循环。",
  lang: "zh-CN",
  cleanUrls: true,
  lastUpdated: true,
  sitemap: { hostname: HOSTNAME },
  head: [
    ["link", { rel: "icon", href: "/favicon.svg", type: "image/svg+xml" }],
    ["meta", { name: "theme-color", content: "#38bdf8" }],
    ["meta", { property: "og:title", content: "YoYoVideo · 全格式本地视频播放器" }],
    [
      "meta",
      {
        property: "og:description",
        content:
          "Rust + Slint + libmpv 打造的跨平台本地视频播放器，支持多画面批量播放、字幕与音轨切换、画面滤镜与 A-B 循环。",
      },
    ],
    ["meta", { property: "og:image", content: `${HOSTNAME}player-default.png` }],
    ["meta", { property: "og:locale", content: "zh_CN" }],
  ],
  themeConfig: {
    logo: "/logo.svg",
    socialLinks: [{ icon: "github", link: REPO }],
    search: {
      provider: "local",
      options: {
        miniSearch: {
          options: {
            // MiniSearch splits on non-alphanumerics, which turns a whole
            // Chinese phrase into one token. Emit per-character tokens too so
            // Chinese search actually returns hits.
            tokenize: (text: string) =>
              text
                .split(/[^\p{L}\p{N}]+/u)
                .flatMap((word) => (/[\u4e00-\u9fa5]/.test(word) ? [word, ...word.split("")] : [word]))
                .filter(Boolean),
            processTerm: (term: string) => term.toLowerCase(),
          },
          searchOptions: { fuzzy: 0.2, prefix: true },
        },
        locales: {
          // localeIndex for the default language is the literal string "root".
          root: {
            translations: {
              button: { buttonText: "搜索文档", buttonAriaLabel: "搜索文档" },
              modal: {
                displayDetails: "显示详细列表",
                resetButtonTitle: "清除查询条件",
                backButtonTitle: "关闭搜索",
                noResultsText: "无法找到相关结果",
                footer: { selectText: "选择", navigateText: "切换", closeText: "关闭" },
              },
            },
          },
        },
      },
    },
  },

  locales: {
    root: {
      label: "简体中文",
      lang: "zh-CN",
      themeConfig: {
        // themeConfig is shallow-merged per locale, so nav/sidebar/footer and
        // every UI string must live inside the locale block in full.
        nav: zhNav,
        sidebar: zhSidebar,
        outline: { level: [2, 3], label: "本页目录" },
        docFooter: { prev: "上一页", next: "下一页" },
        editLink: {
          pattern: `${REPO}/edit/main/docs-site/docs/:path`,
          text: "在 GitHub 上编辑此页",
        },
        lastUpdated: { text: "最后更新于" },
        returnToTopLabel: "回到顶部",
        sidebarMenuLabel: "菜单",
        darkModeSwitchLabel: "外观",
        lightModeSwitchTitle: "切换到浅色模式",
        darkModeSwitchTitle: "切换到深色模式",
        langMenuLabel: "切换语言",
        skipToContentLabel: "跳到主要内容",
        notFound: {
          title: "页面不存在",
          quote: "你访问的页面可能已被移动，或者从未存在过。",
          linkLabel: "返回首页",
          linkText: "返回首页",
        },
        footer: {
          message: "基于 GPL-3.0-or-later 发布，内含 GPL-2.0-or-later 的 mpv / FFmpeg 运行时。",
          copyright: "Copyright © 2026 YoYoVideo",
        },
      },
    },

    en: {
      label: "English",
      lang: "en-US",
      link: "/en/",
      title: "YoYoVideo",
      titleTemplate: ":title | YoYoVideo",
      description:
        "YoYoVideo is a cross-platform local video player built with Rust, Slint and libmpv, with multi-tile batch playback, subtitle and track switching, picture filters and A-B loop.",
      head: [
        [
          "meta",
          {
            name: "keywords",
            content:
              "YoYoVideo,video player,libmpv,mpv,Slint,Rust,local video,media player,batch playback,open source,GPL",
          },
        ],
        ["meta", { property: "og:locale", content: "en_US" }],
      ],
      themeConfig: {
        nav: enNav,
        sidebar: enSidebar,
        outline: { level: [2, 3], label: "On this page" },
        editLink: {
          pattern: `${REPO}/edit/main/docs-site/docs/:path`,
          text: "Edit this page on GitHub",
        },
        lastUpdated: { text: "Last updated" },
        footer: {
          message: "Released under the GPL-3.0-or-later License, with a GPL-2.0-or-later mpv / FFmpeg runtime.",
          copyright: "Copyright © 2026 YoYoVideo",
        },
      },
    },
  },

  // Only genuinely shared keys belong here — anything locale-specific would be
  // clobbered by the shallow merge above.
  transformPageData(pageData) {
    const canonical =
      HOSTNAME +
      pageData.relativePath.replace(/(^|\/)index\.md$/, "$1").replace(/\.md$/, ".html");

    pageData.frontmatter.head ??= [];
    pageData.frontmatter.head.push(
      ["link", { rel: "canonical", href: canonical }],
      ["meta", { property: "og:url", content: canonical }],
      ["meta", { property: "og:title", content: pageData.title ?? "YoYoVideo" }],
      ["meta", { property: "og:description", content: pageData.description ?? "" }],
    );
  },
});
