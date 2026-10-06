---
layout: home
title: YoYoVideo
titleTemplate: 全格式本地视频播放器

hero:
  name: YoYoVideo
  text: 全格式本地视频播放器
  tagline: Rust + Slint + libmpv 打造的轻量播放器。多画面批量播放、字幕与音轨切换、画面滤镜、A-B 循环，全部离线本地完成。
  image:
    src: /player-default.png
    alt: YoYoVideo 启动后的默认界面
  actions:
    - theme: brand
      text: 下载 v0.0.1
      link: https://github.com/ijry/YoYoVideo/releases/latest
    - theme: alt
      text: 快速开始
      link: /guide/installation
    - theme: alt
      text: GitHub
      link: https://github.com/ijry/YoYoVideo

features:
  - title: 多画面批量播放
    details: 一次拖入多个文件即进入批量模式，每个画面独立控制，可拖拽调整单格尺寸。
  - title: 字幕与音轨切换
    details: 外部字幕加载、字幕延迟与缩放调节，音频轨道与声道模式即时切换。
  - title: 画面工具
    details: 缩放、旋转、画面平移，以及一套常用视频滤镜预设。
  - title: A-B 循环与标记
    details: 设定 A/B 两点循环播放，并在时间轴上留下可跳转的标记点。
  - title: 轻量无边框
    details: 自绘无边框播放器界面，鼠标静止时控制条自动隐藏，让画面本身成为全部。
  - title: 纯本地播放内核
    details: 内嵌 libmpv 运行时，无需另外安装 mpv，也不会上传任何数据。
---

<script setup>
import { withBase } from "vitepress";
</script>

<div class="yv-shot-wrap">
  <!-- withBase, not a bare path: VitePress rewrites markdown image syntax but
       leaves raw HTML alone, so a literal src would break under the site base. -->
  <img class="yv-shot" :src="withBase('/player-default.png')" alt="YoYoVideo 默认界面" />
</div>

<section class="yv-section">
  <div class="yv-facts">
    <div class="yv-fact"><b>GPL-3.0</b><span>开源许可</span></div>
    <div class="yv-fact"><b>libmpv</b><span>播放内核</span></div>
    <div class="yv-fact"><b>Rust</b><span>核心语言</span></div>
    <div class="yv-fact"><b>0 依赖</b><span>无需预装解码器</span></div>
  </div>
</section>

<section class="yv-section yv-section-alt">
  <div class="yv-section-inner">
    <h2>为什么是 YoYoVideo</h2>
    <p class="yv-lede">
      市面上的播放器要么功能臃肿，要么在“简洁”的名义下牺牲了专业播放能力。YoYoVideo
      想要的是另一条路：把复杂度放在内核里，把界面收敛到只剩下画面和必要控件。
    </p>

    <div class="yv-grid">
      <div class="yv-card">
        <span class="yv-icon">▶</span>
        <h3>批量播放是一等公民</h3>
        <p>把一个文件夹丢进去，就得到一个可以同时播放的墙面。每个画面都能单独暂停、跳转、调音量，而不是只能切来切去。</p>
      </div>
      <div class="yv-card">
        <span class="yv-icon">⌨</span>
        <h3>为键盘设计</h3>
        <p>快捷键可自定义，播放、进度、音轨、字幕、画面都能不离开键盘完成。滚轮调音量、悬停呼出菜单。</p>
      </div>
      <div class="yv-card">
        <span class="yv-icon">◧</span>
        <h3>无边框，但不简陋</h3>
        <p>标题栏、控制条、侧边栏都收进自绘界面里。窗口静止几秒后界面自动淡出，只留下内容本身。</p>
      </div>
      <div class="yv-card">
        <span class="yv-icon">⚙</span>
        <h3>可复现的构建</h3>
        <p>版本号由 <code>Cargo.toml</code> 唯一决定，打 tag 即发布。运行时来自固定校验和的公开上游构建。</p>
      </div>
    </div>
  </div>
</section>

<section class="yv-section">
  <div class="yv-section-inner">
    <h2>当前进度</h2>
    <p class="yv-lede">v0.0.1 是首个公开版本。以下状态如实反映仓库现状，而不是路线图式承诺。</p>

    <div class="yv-roadmap">
      <div class="yv-step">
        <h4>播放内核与批量播放 <span class="yv-tag">已支持</span></h4>
        <p>libmpv 运行时随包分发，多画面独立控制已可用。</p>
      </div>
      <div class="yv-step">
        <h4>字幕、音轨、画面滤镜 <span class="yv-tag">已支持</span></h4>
        <p>轨道切换、字幕延迟与缩放、常用滤镜预设均已实现。</p>
      </div>
      <div class="yv-step">
        <h4>Windows x64 发行包 <span class="yv-tag">已支持</span></h4>
        <p>便携 zip 与 NSIS 安装包由 GitHub Actions 自动构建并发布。</p>
      </div>
      <div class="yv-step" data-state="next">
        <h4>macOS 发行包 <span class="yv-tag" data-state="next">待完成</span></h4>
        <p>libmpv 取自 Homebrew，两个架构分开构建。视频走 mpv 渲染 API——已实现并通过编译验证，尚未在真机跑过。</p>
      </div>
      <div class="yv-step" data-state="next">
        <h4>Linux 发行包 <span class="yv-tag" data-state="next">待完成</span></h4>
        <p>以 .deb 声明依赖而非捆绑，X11 下视频可用；Wayland 原生嵌入尚未实现。</p>
      </div>
    </div>
  </div>
</section>

<section class="yv-section yv-cta">
  <div class="yv-section-inner">
    <h2>下载 v0.0.1</h2>
    <p class="yv-lede">
      Windows x64 便携包与安装包。解压即用，不需要预装任何解码器。
    </p>
    <div class="yv-actions">
      <a class="yv-btn yv-btn-primary" href="https://github.com/ijry/YoYoVideo/releases/latest">前往下载页</a>
      <a class="yv-btn yv-btn-ghost" href="https://github.com/ijry/YoYoVideo">查看源码</a>
      <a class="yv-btn yv-btn-ghost" :href="withBase('/guide/installation')">安装说明</a>
    </div>
  </div>
</section>
