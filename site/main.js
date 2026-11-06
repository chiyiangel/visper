(() => {
  const zh = {
    "nav.features": "功能", "nav.how": "工作原理", "nav.models": "模型", "nav.build": "构建",
    "hero.eyebrow": "开源 · macOS 与 Windows",
    "hero.title": "录音变文字，<br><em>只留在你的电脑上。</em>",
    "hero.lede": "Visper 用 Whisper large-v3 把会议、课程、访谈录音转成带时间戳的转写稿，完全离线。中文、英文，或同一份录音里中英混杂都可以。",
    "cta.github": "在 GitHub 上查看", "cta.star": "给项目点个 Star", "cta.contribute": "参与贡献",
    "soon.pill": "即将推出", "soon.text": "预编译安装包正在准备中。现在可以用几条命令从源码构建。",
    "ui.library": "资料库",
    "ui.caption": "界面示意图（应用界面为简体中文），示例文字来自应用自带的预览数据。",
    "facts.offline.t": "100% 离线", "facts.offline.d": "只需下载一次模型", "facts.model.d": "或 large-v3-turbo，未量化",
    "facts.gpu.d": "GPU 不可用时自动退回 CPU", "facts.rust.d": "原生应用，MIT OR Apache-2.0",
    "features.kicker": "功能", "features.title": "转写稿需要的都有，不需要的一概不加。",
    "features.lede": "为你已有的录音而做：中文、英文，或两者混杂。",
    "f.offline.t": "隐私优先", "f.offline.d": "模型、录音和转写稿都留在你的电脑上。唯一的联网行为，是首次从 Hugging Face 下载模型。",
    "f.mixed.t": "中英混杂", "f.mixed.d": "“自动”模式按静音切段，逐段判断中文或英文，会议中途换语言也不会把后面带偏。也可以把整份录音固定为一种语言。",
    "f.simplified.t": "输出简体中文", "f.simplified.d": "繁体字在显示和保存之前，用 OpenCC 转成简体。",
    "f.gpu.t": "GPU 加速", "f.gpu.d": "macOS 用 Metal，Windows 用 Vulkan，开启 flash attention。GPU 初始化失败时，这次转写自动改用 CPU。",
    "f.queue.t": "队列式转写", "f.queue.d": "想加多少录音都行，按顺序一份份转写，实时显示分段、进度、速度和剩余时间。",
    "f.play.t": "点击回听", "f.play.d": "每个分段都有时间戳，点一下就从那个位置开始播放原录音。",
    "f.export.t": "导出 Markdown", "f.export.d": "每个分段一块 <code>[HH:MM:SS] 文字</code>，可直接放进笔记、文档，或交给你常用的大模型。",
    "f.library.t": "资料库", "f.library.d": "转写稿在重启后仍在。可重新转写、取消或移除。录音文件被移走，转写稿依然可读。Visper 从不复制你的音频。",
    "mixed.kicker": "逐段语言判断", "mixed.title": "一句中文一句英文，Visper 都跟得上。",
    "mixed.p1": "Whisper 自带的自动识别会给整份文件只猜一种语言，后面换成另一种语言的部分就会被带偏。Visper 则在每两个停顿之间的语音段上分别判断中文还是英文，再按各自的语言转写。",
    "mixed.p2": "英文保持英文，中文保持中文，从不翻译。",
    "mixed.link": "阅读设计决策（ADR 0002）→",
    "how.kicker": "工作原理", "how.title": "从音频文件到转写稿，五步完成",
    "how.1.t": "解码", "how.1.d": "用 Symphonia 解码 m4a、mp3、wav、flac、aac，再用 Rubato 重采样为 16 kHz 单声道。",
    "how.2.t": "按静音切段", "how.2.d": "以 20 ms 为一帧测响度，阈值随底噪自适应。400 ms 以上的停顿把语音分成每段不超过 30 秒。",
    "how.3.t": "判断语言", "how.3.d": "自动模式下，每段判为中文或英文；相邻的同语言段合并进同一个窗口。",
    "how.4.t": "转写", "how.4.d": "whisper.cpp 以 beam search 在 Metal 或 Vulkan 上推理，必要时退回 CPU。",
    "how.5.t": "转简体并保存", "how.5.d": "OpenCC 转成简体中文；分段实时推送到窗口，并保存进本机资料库。",
    "models.kicker": "两个模型", "models.title": "要准还是要快，随时切换。",
    "models.lede": "首次启动时选择，之后在资料库底部切换。每份转写稿都会记下产生它的模型。",
    "m.large.tag": "准确度最高", "m.large.d": "适合专有名词多的访谈。",
    "m.turbo.tag": "快得多", "m.turbo.d": "准确度略低，适合日常会议。解码器从 32 层精简到 4 层。",
    "models.note": "M3 Pro 上的实测：Metal 推理比经 MoltenVK 的 Vulkan 快约 1.6 倍，内存少用约 0.5 GB。",
    "build.kicker": "从源码构建", "build.title": "几条命令就能跑起来",
    "build.p1": "需要 Rust stable（通过 rust-toolchain.toml 自动安装）和 CMake。首次启动时，Visper 会让你选择要下载的模型。",
    "build.r1": "Apple Silicon 的 macOS，或 x86_64 的 Windows 10/11",
    "build.r2": "Windows：MSVC 构建工具、CMake 和 LunarG Vulkan SDK",
    "build.r3": "模型需要 1.6–3.1 GB 磁盘空间",
    "build.soon": "两个平台的预编译安装包。", "build.copy": "复制",
    "scope.kicker": "有意保持小巧", "scope.title": "Visper 不做的事",
    "scope.lede": "专注把一件事做好。以下功能是刻意不做的：",
    "scope.1": "麦克风实时录音", "scope.2": "翻译", "scope.3": "摘要", "scope.4": "说话人标注",
    "scope.5": "编辑转写稿", "scope.6": "自定义提示词或词表", "scope.7": "中英文以外的语言",
    "final.title": "开源，并将一直开源。",
    "final.lede": "采用 MIT 或 Apache-2.0 双许可。欢迎用中文或英文提 Issue 和 PR。",
    "foot.license": "MIT OR Apache-2.0", "foot.changelog": "更新日志", "foot.canvas": "设计画布",
    "foot.credit": "基于 OpenAI Whisper、whisper.cpp 和 GPUI 构建。"
  };

  const html = document.documentElement;
  const nodes = [...document.querySelectorAll("[data-i18n],[data-i18n-html]")];
  const en = new Map(nodes.map((n) => [n, n.innerHTML]));
  const langBtn = document.getElementById("lang");
  const titles = { en: document.title, zh: "Visper · 在自己电脑上离线转写中英文录音" };

  function apply(lang) {
    const isZh = lang === "zh";
    html.lang = isZh ? "zh-CN" : "en";
    for (const n of nodes) {
      const key = n.dataset.i18n || n.dataset.i18nHtml;
      n.innerHTML = isZh && zh[key] ? zh[key] : en.get(n);
    }
    document.title = titles[isZh ? "zh" : "en"];
    langBtn.textContent = isZh ? "English" : "中文";
    langBtn.setAttribute("aria-label", isZh ? "Switch to English" : "切换到中文");
    try { localStorage.setItem("visper-lang", lang); } catch {}
  }

  let initial = new URLSearchParams(location.search).get("lang");
  if (!initial) { try { initial = localStorage.getItem("visper-lang"); } catch {} }
  if (!initial) initial = (navigator.language || "").toLowerCase().startsWith("zh") ? "zh" : "en";
  apply(initial);
  langBtn.addEventListener("click", () => apply(html.lang === "zh-CN" ? "en" : "zh"));

  // Build tabs + copy
  const tabs = document.querySelectorAll(".tabs [data-tab]");
  const panes = document.querySelectorAll(".tab-pane");
  tabs.forEach((t) => t.addEventListener("click", () => {
    tabs.forEach((x) => x.setAttribute("aria-selected", String(x === t)));
    panes.forEach((p) => (p.hidden = p.dataset.pane !== t.dataset.tab));
  }));
  const copy = document.querySelector(".tabs .copy");
  copy.addEventListener("click", async () => {
    const pane = [...panes].find((p) => !p.hidden);
    const text = pane.innerText.split("\n").filter((l) => l.trim() && !l.trim().startsWith("#")).join("\n");
    try {
      await navigator.clipboard.writeText(text);
      const old = copy.textContent; copy.textContent = "✓"; setTimeout(() => (copy.textContent = old), 1200);
    } catch {}
  });

  // Reveal on scroll
  const targets = document.querySelectorAll(".card, .pipeline li, .model, .md-card, .term, .chips li");
  if ("IntersectionObserver" in window && !matchMedia("(prefers-reduced-motion: reduce)").matches) {
    targets.forEach((t) => t.classList.add("reveal"));
    const io = new IntersectionObserver((entries) => entries.forEach((e) => {
      if (e.isIntersecting) { e.target.classList.add("in"); io.unobserve(e.target); }
    }), { rootMargin: "0px 0px -8% 0px" });
    targets.forEach((t) => io.observe(t));
  }
})();
