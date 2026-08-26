from __future__ import annotations

import json
import shutil
import sqlite3
import struct
from datetime import datetime, timedelta
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
WORK = Path(__file__).resolve().parent
RUNTIME = WORK / "runtime"
DATA = RUNTIME / "data"
DEMO_FILES = DATA / "demo-files"
IMAGES = DATA / "images"
EXE_SRC = ROOT / "target" / "release" / "zsclip.exe"
EXE_DST = RUNTIME / "zsclip.exe"


def write_bmp(path: Path, width: int, height: int) -> None:
    row_size = width * 4
    pixel_size = row_size * height
    file_size = 14 + 40 + pixel_size
    header = b"BM" + struct.pack("<IHHI", file_size, 0, 0, 54)
    dib = struct.pack(
        "<IIIHHIIIIII",
        40,
        width,
        height,
        1,
        32,
        0,
        pixel_size,
        2835,
        2835,
        0,
        0,
    )
    pixels = bytearray()
    for y in range(height - 1, -1, -1):
        for x in range(width):
            band = int(255 * x / max(1, width - 1))
            shade = int(255 * y / max(1, height - 1))
            if 24 < x < width - 24 and 24 < y < height - 24:
                r, g, b = 28, 110 + band // 4, 210
            else:
                r, g, b = 246 - shade // 8, 248 - shade // 10, 252
            pixels.extend((b, g, r, 255))
    path.write_bytes(header + dib + bytes(pixels))


def ensure_runtime() -> None:
    if not EXE_SRC.exists():
        raise SystemExit(f"missing built executable: {EXE_SRC}")
    DATA.mkdir(parents=True, exist_ok=True)
    DEMO_FILES.mkdir(parents=True, exist_ok=True)
    IMAGES.mkdir(parents=True, exist_ok=True)
    shutil.copy2(EXE_SRC, EXE_DST)


def create_demo_files() -> tuple[Path, Path]:
    contract = DEMO_FILES / "项目A-合同条款摘录.txt"
    invoice = DEMO_FILES / "发票资料-6月.xlsx"
    contract.write_text(
        "甲方确认：本项目交付节点为 2026-06-30，验收资料以 WPS 文档清单为准。\n",
        encoding="utf-8",
    )
    invoice.write_text("演示文件，不包含真实财务数据。\n", encoding="utf-8")
    return contract, invoice


def seed_database(contract: Path, invoice: Path, screenshot: Path) -> None:
    db = DATA / "clipboard.db"
    if db.exists():
        db.unlink()
    for suffix in ("-wal", "-shm"):
        extra = DATA / f"clipboard.db{suffix}"
        if extra.exists():
            extra.unlink()

    conn = sqlite3.connect(db)
    conn.executescript(
        """
        CREATE TABLE items(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            category INTEGER NOT NULL,
            kind TEXT NOT NULL,
            preview TEXT NOT NULL,
            signature TEXT NOT NULL DEFAULT '',
            text_data TEXT,
            source_app TEXT NOT NULL DEFAULT '',
            file_paths TEXT,
            image_data BLOB,
            image_path TEXT,
            image_width INTEGER NOT NULL DEFAULT 0,
            image_height INTEGER NOT NULL DEFAULT 0,
            pinned INTEGER NOT NULL DEFAULT 0,
            group_id INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE clip_groups(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            category INTEGER NOT NULL DEFAULT 0,
            name TEXT NOT NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX idx_items_category_pinned_id ON items(category, pinned, id DESC);
        CREATE INDEX idx_items_group_id ON items(group_id, id DESC);
        CREATE INDEX idx_items_category_signature ON items(category, signature, id DESC);
        CREATE INDEX idx_clip_groups_category_sort ON clip_groups(category, sort_order, id);
        CREATE UNIQUE INDEX idx_clip_groups_category_name ON clip_groups(category, name);
        """
    )

    def group(category: int, name: str, order: int) -> int:
        cur = conn.execute(
            "INSERT INTO clip_groups(category, name, sort_order) VALUES(?, ?, ?)",
            (category, name, order),
        )
        return int(cur.lastrowid)

    record_groups = {
        "项目资料": group(0, "项目资料", 1),
        "客户素材": group(0, "客户素材", 2),
        "发票合同": group(0, "发票合同", 3),
        "截图识别": group(0, "截图识别", 4),
    }
    phrase_groups = {
        "客服回复": group(1, "客服回复", 1),
        "合同模板": group(1, "合同模板", 2),
        "地址账号": group(1, "地址账号", 3),
    }

    now = datetime.now().replace(microsecond=0)

    def insert_item(
        category: int,
        kind: str,
        preview: str,
        text: str | None,
        source: str,
        group_id: int,
        minutes_ago: int,
        pinned: int = 0,
        file_paths: str | None = None,
        image_path: str | None = None,
        width: int = 0,
        height: int = 0,
    ) -> None:
        created_at = (now - timedelta(minutes=minutes_ago)).strftime("%Y-%m-%d %H:%M:%S")
        signature = f"demo:{category}:{kind}:{preview}:{minutes_ago}"
        conn.execute(
            """
            INSERT INTO items(category, kind, preview, signature, text_data, source_app, file_paths,
                              image_path, image_width, image_height, pinned, group_id, created_at)
            VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                category,
                kind,
                preview[:120],
                signature,
                text,
                source,
                file_paths,
                image_path,
                width,
                height,
                pinned,
                group_id,
                created_at,
            ),
        )

    records = [
        ("合同付款条款：首付款 30%，验收后 60%，质保金 10%。", "WPS Writer", "发票合同", 8, 1),
        ("客户回复模板：您好，资料已收到，我这边核对后今天下午反馈。", "微信", "客户素材", 16, 0),
        ("发票抬头：筑森演示科技有限公司，税号 91320000DEMO2026。", "WPS表格", "发票合同", 24, 0),
        ("会议纪要：下周一确认 UI 截图、token 使用量截图和 OCR 演示素材。", "WPS Writer", "项目资料", 36, 0),
        ("cargo check -p zsclip 通过，下一步补 Remotion 剪辑工程。", "Visual Studio Code", "项目资料", 48, 0),
        ("OCR 识别结果：请在 6 月 30 日前提交盖章扫描件。", "截图工具", "截图识别", 55, 0),
        ("build in bilibili AI 创造公开赛：产品讲功能，也讲构建过程。", "Edge", "项目资料", 66, 0),
        ("WebDAV 与局域网同步保持单选，局域网默认关闭，用时启动。", "Visual Studio Code", "项目资料", 78, 0),
        ("WPS 任务窗格：搜索本机剪贴板记录，一点插入当前光标。", "WPS Writer", "项目资料", 90, 0),
        ("VV 模式演示短语：输入 vv 后按 1 到 9 直接粘贴。", "记事本", "客户素材", 105, 0),
        ("日期搜索演示：今天复制的发票、合同、WPS 记录都能筛出来。", "WPS表格", "发票合同", 120, 0),
        ("搜索附近后续计划：命中记录前后时间线一起展示。", "Visual Studio Code", "项目资料", 135, 0),
    ]
    for text, source, group_name, minutes, pinned in records:
        insert_item(0, "text", text, text, source, record_groups[group_name], minutes, pinned)

    insert_item(
        0,
        "files",
        f"{contract.name}\n{invoice.name}",
        None,
        "Explorer",
        record_groups["发票合同"],
        150,
        file_paths=f"{contract}\n{invoice}",
    )
    insert_item(
        0,
        "image",
        "截图 OCR 演示图：合同盖章扫描件",
        None,
        "截图工具",
        record_groups["截图识别"],
        165,
        image_path=str(screenshot),
        width=640,
        height=360,
    )

    phrases = [
        ("您好，资料已收到，我会在今天下午 5 点前给您反馈。", "客服回复", 5),
        ("如需开票，请提供公司名称、税号、地址电话、开户行及账号。", "客服回复", 12),
        ("本合同附件与正文具有同等法律效力，请双方确认后盖章。", "合同模板", 18),
        ("项目交付资料清单：合同、发票、验收单、截图说明、操作记录。", "合同模板", 25),
        ("公司地址：江苏省南京市演示路 88 号，收件人：ZSClip 测试。", "地址账号", 32),
        ("售后说明：如 macOS / Linux 初代版本遇到问题，请加群反馈。", "客服回复", 40),
    ]
    for text, group_name, minutes in phrases:
        insert_item(
            1,
            "phrase",
            text,
            text,
            "常用短语",
            phrase_groups[group_name],
            minutes,
        )

    conn.commit()
    conn.close()


def write_settings() -> None:
    settings = {
        "hotkey_enabled": True,
        "hotkey_mod": "Win",
        "hotkey_key": "V",
        "silent_start": False,
        "tray_icon_enabled": True,
        "close_without_exit": True,
        "clipboard_capture_enabled": False,
        "max_items": 5000,
        "show_pos_mode": "fixed",
        "show_fixed_x": 160,
        "show_fixed_y": 120,
        "quick_search_enabled": True,
        "vv_mode_enabled": True,
        "vv_source_tab": 1,
        "vv_group_id": 0,
        "image_preview_enabled": True,
        "quick_delete_button": True,
        "dedupe_filter_enabled": False,
        "persistent_search_box": True,
        "search_engine": "bing",
        "search_template": "https://www.bing.com/search?q={q}",
        "ai_clean_enabled": True,
        "super_mail_merge_enabled": True,
        "wps_taskpane_enabled": True,
        "grouping_enabled": True,
        "cloud_sync_enabled": False,
        "lan_sync_enabled": False,
        "lan_tcp_port": 38473,
        "lan_udp_port": 38472,
        "lan_last_status": "演示数据：局域网同步未启动",
        "lan_receive_mode": "records_only",
        "image_ocr_provider": "wechat",
        "text_translate_provider": "off",
        "text_translate_target_lang": "zh",
        "qr_quick_enabled": True,
    }
    (DATA / "settings.json").write_text(
        json.dumps(settings, ensure_ascii=False, indent=2), encoding="utf-8"
    )


def main() -> None:
    ensure_runtime()
    contract, invoice = create_demo_files()
    screenshot = IMAGES / "ocr-contract-demo.bmp"
    write_bmp(screenshot, 640, 360)
    seed_database(contract, invoice, screenshot)
    write_settings()
    print(f"runtime={RUNTIME}")
    print(f"exe={EXE_DST}")
    print(f"data={DATA}")


if __name__ == "__main__":
    main()
