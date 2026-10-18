<div align="center">

<img src="packaging/icons/zeronet.png" alt="ZeroNet" width="170">

# ZeroNet Desktop

### Performance, Speed, and Freedom in your hands.
### کارایی، سرعت و آزادی در دستان شما.

<a href="https://github.com/ZeroNetTeam/ZeroNet-Desktop/releases/latest"><b>⬇️ دانلود آخرین نسخه</b></a><br>
<a href="https://github.com/ZeroNetTeam/ZeroNet-Desktop/releases/latest"><b>⬇️ Download the latest release</b></a>

[فارسی](#فارسی) · [English](#english)

</div>

---

<div dir="rtl">

## فارسی

**زیرونت دسکتاپ** نسخه‌ی ویندوز و لینوکس زیرونت است، یک VPN ساده و سریع برای عبور از فیلترینگ. یک برنامه‌ی ترمینالی است که مثل برنامه‌ی دسکتاپ کار می‌کند: موس، منو و کلیک. قلب آن [Zray Core](https://github.com/ZeroNetTeam/Zray-Core) است، یک هسته‌ی شبکه که از صفر با **Rust** نوشته شده و با کانفیگ‌های Xray (VLESS،&rlm; REALITY،&rlm; Vision،&rlm; XHTTP،&rlm; VMess،&rlm; Trojan،&rlm; Shadowsocks و…) کار می‌کند.

نسخه‌ی اندروید در [ZeroNet-Mobile](https://github.com/ZeroNetTeam/ZeroNet-Mobile) است و نسخه‌ی گرافیکی دسکتاپ، [Z-UI](https://github.com/ZeroNetTeam/Z-UI)، در دست ساخت است. سرعت و مصرف رم و CPU هسته در [Zray Core](https://github.com/ZeroNetTeam/Zray-Core#readme) اندازه‌گیری و منتشر شده است.

### 📥 نصب (فقط چند کلیک)

همه‌ی فایل‌ها در صفحه‌ی **[Releases](https://github.com/ZeroNetTeam/ZeroNet-Desktop/releases/latest)** هستند:

| سیستم | فایل | چه کار کنم؟ |
|---|---|---|
| 🪟 ویندوز | `ZeroNet-Windows-x64.zip` | فایل را از حالت فشرده خارج کنید و روی `ZeroNet.exe` دوبار کلیک کنید. اگر ویندوز هشدار داد: **More info ←&rlm; Run anyway**. |
| 🐧 لینوکس (بیشتر کامپیوترها: اینتل و AMD) | `ZeroNet-Linux-x64.AppImage` | راست‌کلیک ← Properties ← تیک **Allow executing file as program**. بعد دوبار کلیک کنید. برنامه خودش در ترمینال باز می‌شود. |
| 🐧 لینوکس روی ARM (رزبری‌پای، لپ‌تاپ‌های ARM) | `ZeroNet-Linux-ARM64.AppImage` | مثل بالا. |

> مطمئن نیستید کدام فایل لینوکس؟ در ترمینال `uname -m` را بزنید: `x86_64` یعنی **x64** و `aarch64` یعنی **ARM64**. خطای «exec format error» یعنی فایل آن یکی را لازم دارید.

### 🧭 استفاده

1. لینک کانفیگ (`vless://`،&rlm; `vmess://`،&rlm; `trojan://`،&rlm; `ss://` یا لینک ساب‌اسکریپشن) را کپی کنید و در زیرونت **Ctrl+V** بزنید.
2. روی دکمه‌ی اتصال کلیک کنید. همه‌چیز با موس کار می‌کند، مثل یک برنامه‌ی معمولی.
3. در تنظیمات، **System Proxy** را روی **SET SYSTEM** بگذارید (یا آن‌قدر **Ctrl+P** بزنید تا SET SYSTEM شود) تا بقیه‌ی برنامه‌ها هم از آن استفاده کنند.

**اکانت رایگان WARP:** توی زیرونت کلید **W** را بزنید تا یک اکانت WARP از کلودفلر بگیرید. کلیدها روی دستگاه خودتان ساخته می‌شوند و فقط نیمه‌ی عمومی‌شان فرستاده می‌شود. هر سه راه (WireGuard و MASQUE روی HTTP/2 و روی HTTP/3) را با هم امتحان می‌کند و اولی را نگه می‌دارد که واقعاً ترافیک رد کند، و یادش می‌ماند کدام روی شبکه‌ی شما کار کرد. اولین بار که وصل می‌شوید، زیرونت می‌پرسد اجازه دارید از کلودفلر استفاده کند یا نه، و جواب شما یادش می‌ماند (از تنظیمات، بخش **Cloudflare WARP**) و عوض می‌شود. اگر API کلودفلر جایی که هستید فیلتر باشد، خودش اول یک سرور بالا می‌آورد، از راه آن اکانت را می‌سازد، آن سرور را رها می‌کند و بعد اکانت را با ترتیب WARP که انتخاب کرده‌اید وصل می‌کند. با **W** می‌توانید هر وقت خواستید دستی هم این کار را بکنید.

**برنامه‌های دیگر:**
- **تلگرام:** Settings ←&rlm; Advanced ←&rlm; Connection type ←&rlm; **Use system proxy**. در لینوکس بعد از روشن‌کردن پروکسی، تلگرام را کامل ببندید و دوباره باز کنید.
- **فایرفاکس:** Settings ←&rlm; Network Settings ←&rlm; **Use system proxy settings**.

**حالت TUN** (همه‌ی برنامه‌ها بدون تنظیم جداگانه): در لینوکس و مک زیرونت رمز سیستم را می‌پرسد. در ویندوز برنامه را با **Run as administrator** باز کنید (فایل `wintun.dll` کنار برنامه است).

### 🛠️ ساخت از سورس

</div>

```sh
cargo run --release -p zeronet-tui
```

برای کار هم‌زمان روی هسته و برنامه، [Zray-Core/tools/link-local.sh](https://github.com/ZeroNetTeam/Zray-Core/blob/main/tools/link-local.sh) را ببینید.

---

## English

**ZeroNet Desktop** is ZeroNet for Windows and Linux, a fast, simple VPN for
getting past censorship. It is a terminal app that works like a desktop app:
mouse, hover, menus, clicks. When you double-click it, it opens its own
terminal window. Its heart is [Zray Core](https://github.com/ZeroNetTeam/Zray-Core),
a networking core written from scratch in **Rust** that speaks the same configs
as Xray: VLESS, REALITY, Vision, XHTTP, VMess, Trojan, Shadowsocks and more.

The Android app is [ZeroNet-Mobile](https://github.com/ZeroNetTeam/ZeroNet-Mobile)
and the graphical desktop app, [Z-UI](https://github.com/ZeroNetTeam/Z-UI), is
in the works. The speed and memory measurements of the core are published with
[Zray Core](https://github.com/ZeroNetTeam/Zray-Core#readme).

### 📥 Install

Everything is on the **[Releases page](https://github.com/ZeroNetTeam/ZeroNet-Desktop/releases/latest)**:

| Platform | File | What to do |
|---|---|---|
| 🪟 Windows | `ZeroNet-Windows-x64.zip` | Extract it and double-click `ZeroNet.exe`. If SmartScreen appears: **More info → Run anyway**. |
| 🐧 Linux, most computers (Intel / AMD) | `ZeroNet-Linux-x64.AppImage` | Right-click → Properties → **Allow executing file as program**, then double-click. It opens in a terminal window on its own. (A plain `.tar.gz` is there too.) |
| 🐧 Linux on ARM (Raspberry Pi, ARM laptops) | `ZeroNet-Linux-ARM64.AppImage` | The same. |

Not sure which Linux file? Run `uname -m` in a terminal: `x86_64` means
**x64**, `aarch64` means **ARM64**. "exec format error" means you have the
other one.

### 🧭 Using it

1. Copy a config link (`vless://`, `vmess://`, `trojan://`, `ss://`, or a
   subscription URL) and press **Ctrl+V** in ZeroNet.
2. Click connect.
3. In Settings, set **System Proxy** to **SET SYSTEM** (or press **Ctrl+P**
   until it shows SET SYSTEM) so other apps use it too.

**Free WARP account:** press **W** in ZeroNet to get a Cloudflare WARP account. The keys are made on your device and only the public halves are sent. It tries all three ways (WireGuard, MASQUE over HTTP/2 and over HTTP/3) at once and keeps the first one that really carries traffic, and remembers which one worked on your network. The first time you connect, ZeroNet asks whether it may use Cloudflare and remembers the answer (change it under Settings, Cloudflare WARP). If Cloudflare's API is filtered where you are, it brings a server up itself, makes the account through that tunnel, lets the server go again and then connects the account in the WARP order you picked. Press **W** to do it by hand at any time.

**Other apps:**
- **Telegram:** Settings → Advanced → Connection type → **Use system proxy**.
  On Linux, Telegram reads the proxy only when it starts, so fully quit it
  and open it again after turning the proxy on.
- **Firefox:** Settings → Network Settings → **Use system proxy settings**.
- **TUN mode** covers every app without configuring anything. On Linux and
  macOS, ZeroNet asks for your password. On Windows, start it with **Run as
  administrator**; the `wintun.dll` it needs ships in the zip.

### 🛠️ Build from source

```sh
cargo run --release -p zeronet-tui
```

The engine is a git dependency on [Zray-Core](https://github.com/ZeroNetTeam/Zray-Core),
pinned by `Cargo.lock`. To work on both at once, clone them side by side and
run `Zray-Core/tools/link-local.sh ZeroNet-Desktop`; delete
`.cargo/config.toml` to go back to the pinned revision.

Maintainers: pushing a tag like `v0.2.0` builds the downloads and publishes the
release (`.github/workflows/release.yml`).

### Repository layout

| Path | What it is |
|---|---|
| `crates/zeronet-tui/` | The app: terminal interface, background service, local database, system proxy, updater. |
| `packaging/` | Icons and desktop metadata used by the release builds. |
| `.github/` | CI and the release build. |

### License

GPL-3.0 (see `LICENSE`). Third-party components keep their own licences.
