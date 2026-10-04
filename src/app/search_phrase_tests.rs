use super::*;

fn query(text: &str, category: i64) -> ItemsQuery {
    ItemsQuery { category, group_id: 0, search_text: text.into(), kind_filter: ClipKindFilter::All, near_query: None }
}

fn insert(text: &str, title: &str, category: i64) -> rusqlite::Result<i64> {
    with_db(|conn| {
        conn.execute("INSERT INTO items(category,kind,preview,text_data,phrase_title,source_app,created_at) VALUES(?1,?2,?3,?3,?4,'SyntheticEditor','2026-10-05 12:00:00')",
            params![category, if category == 1 { "phrase" } else { "text" }, text, title])?;
        Ok(conn.last_insert_rowid())
    })
}

#[test]
fn phrase_title_body_roundtrip_search_and_format_preservation() {
    crate::db_runtime::with_test_protected_texts(&[], || crate::db_runtime::with_test_db(|| {
        let id = insert("正文保持原样\n第二行", "项目😀标题", 1)?;
        let second = insert("正文保持原样\n第二行", "其他标题", 1)?;
        with_db(|conn| conn.execute("UPDATE items SET rich_text_html='<b>正文保持原样</b>' WHERE id=?", [id]))?;
        let item = db_load_item_full(id).unwrap();
        assert_eq!(item.display_title(), "项目😀标题");
        assert_eq!(item.text.as_deref(), Some("正文保持原样\n第二行"));
        assert_eq!(db_load_items_page(&query("项目😀", 1), None, 200)?.0[0].id, id);
        assert_eq!(db_load_items_page(&query("第二行", 1), None, 200)?.0.len(), 2);
        db_save_phrase(id, "改名", item.text.as_deref().unwrap()).unwrap();
        let renamed = db_load_item_full(id).unwrap();
        assert_eq!(renamed.rich_text_html, item.rich_text_html);
        assert_eq!(renamed.phrase_title, "改名");
        db_save_phrase(id, "", "修改后正文").unwrap();
        let edited = db_load_item_full(id).unwrap();
        assert_eq!(edited.display_title(), "修改后正文");
        assert!(edited.rich_text_html.is_none());
        assert!(db_find_duplicate_item_ids(1, &item, "same").is_empty());
        assert_eq!(db_reconcile_dedupe_signatures_impl(1, false)?, 0);
        assert!(db_load_item_full(second).is_some());
        assert!(crate::app_core::normalize_phrase_title(&"😀".repeat(60)).is_ok());
        assert!(crate::app_core::normalize_phrase_title(&"字".repeat(61)).is_err());
        assert!(crate::app_core::normalize_phrase_title("标题\n换行").is_err());
        Ok(())
    })).unwrap();
}

#[test]
fn search_literals_unicode_filters_and_keyset_pages_are_complete() {
    crate::db_runtime::with_test_protected_texts(&[], || crate::db_runtime::with_test_db(|| {
        let literal = insert(r"中文甲 😀 100% a_b C:\资料\file", "", 0)?;
        let other = insert("中文乙 1000 axb other", "", 0)?;
        for term in ["甲", "中文甲", "😀", "100%", "a_b", r"C:\资料"] {
            let rows = db_load_items_page(&query(term, 0), None, 200)?.0;
            assert_eq!(rows.iter().map(|v| v.id).collect::<Vec<_>>(), vec![literal], "{term}");
        }
        let (first, cursor, more) = db_load_items_page(&query("中文", 0), None, 1)?;
        assert!(more);
        assert_eq!(first[0].id, other);
        let (second, _, more) = db_load_items_page(&query("中文", 0), cursor, 1)?;
        assert!(!more);
        assert_eq!(second[0].id, literal);
        assert_eq!(db_load_items_page(&query("应用:syntheticeditor 中文", 0), None, 200)?.0.len(), 2);
        assert!(db_load_items_page(&query("没有这样的内容", 0), None, 200)?.0.is_empty());
        let local_date: String = with_db(|conn| conn.query_row("SELECT date(created_at,'localtime') FROM items WHERE id=?", [literal], |row| row.get(0)))?;
        assert_eq!(db_load_items_page(&query(&format!("日期:{local_date} 中文"), 0), None, 200)?.0.len(), 2);
        Ok(())
    })).unwrap();
}

#[test]
fn promoting_phrase_preserves_title_body_format_and_group() {
    crate::db_runtime::with_test_protected_texts(&[], || crate::db_runtime::with_test_db(|| {
        let id = insert("只粘贴正文", "提升后仍保留😀标题", 1)?;
        with_db(|conn| {
            conn.execute("UPDATE items SET rich_text_html='<b>只粘贴正文</b>', group_id=17, pinned=1 WHERE id=?", [id])?;
            Ok(())
        })?;
        let before = db_load_item_full(id).unwrap();
        let promoted = db_promote_item_to_top(id)?;
        assert_ne!(id, promoted);
        assert!(db_load_item_full(id).is_none());
        let after = db_load_item_full(promoted).unwrap();
        assert_eq!(after.phrase_title, before.phrase_title);
        assert_eq!(after.text, before.text);
        assert_eq!(after.rich_text_html, before.rich_text_html);
        assert_eq!(after.group_id, before.group_id);
        assert_eq!(after.pinned, before.pinned);
        assert_eq!(db_load_items_page(&query("提升后仍保留", 1), None, 200)?.0[0].id, promoted);
        Ok(())
    })).unwrap();
}

#[test]
fn cached_title_protection_is_checked_and_bad_html_keeps_plain_body() {
    crate::db_runtime::with_test_protected_texts(&[], || crate::db_runtime::with_test_db(|| {
        let id = insert("ordinary body", "becomes-protected", 1)?;
        let cached = db_load_item_full(id).unwrap();
        crate::db_runtime::with_test_protected_texts(&["becomes-protected"], || {
            assert!(crate::app::state_runtime::protected_item_for_use(cached.clone()).is_none());
        });
        let mut bad_html = cached;
        bad_html.phrase_title.clear();
        bad_html.rich_text_html = Some("<b>ordinary html-secret ordinary</b>".into());
        bad_html.preview = "stale secret preview".into();
        crate::db_runtime::with_test_protected_texts(&["html-secret"], || {
            let safe = crate::app::state_runtime::protected_item_for_use(bad_html).unwrap();
            assert_eq!(safe.text.as_deref(), Some("ordinary body"));
            assert!(safe.rich_text_html.is_none());
            assert_eq!(safe.preview, build_preview("ordinary body"));
        });
        Ok(())
    })).unwrap();
}

#[test]
fn running_search_is_cancelled_and_connection_remains_usable() {
    crate::db_runtime::with_test_db(|| with_db(|conn| {
        const WINDOW: isize = 99_109_001;
        mark_latest_page_request(WINDOW, 0, 1);
        conn.create_scalar_function("cancel_search", 0, rusqlite::functions::FunctionFlags::SQLITE_UTF8,
            |_| { mark_latest_page_request(WINDOW, 0, 2); Ok(1_i64) })?;
        let result = run_latest_search(conn, WINDOW, 0, 1, || {
            conn.query_row("WITH RECURSIVE numbers(x) AS (SELECT cancel_search() UNION ALL SELECT x+1 FROM numbers WHERE x<10000000) SELECT sum(x) FROM numbers", [], |row| row.get::<_,i64>(0))
        })?;
        assert!(result.is_none());
        assert_eq!(conn.query_row("SELECT 42", [], |row| row.get::<_,i64>(0))?, 42);
        Ok(())
    })).unwrap();
}

#[test]
fn protection_change_discards_search_snapshot_and_titles_are_filtered() {
    let registry = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    crate::db_runtime::with_test_protection_registry(registry.clone(), || crate::db_runtime::with_test_db(|| {
        insert("ordinary body", "secret title", 1)?;
        let changed = crate::db_runtime::with_search_protection(|| {
            registry.lock().unwrap().push("secret title".into());
            assert!(!crate::db_runtime::text_is_protected("secret title"));
            Ok(())
        });
        assert!(changed.is_err());
        assert!(db_load_items_page(&query("", 1), None, 200)?.0.is_empty());
        Ok(())
    })).unwrap();
}

#[test]
fn phrase_title_is_kept_in_summaries_and_legacy_body_is_unchanged() {
    crate::db_runtime::with_test_protected_texts(&[], || crate::db_runtime::with_test_db(|| {
        let id = insert("legacy body", "", 1)?;
        with_db(|conn| {
            conn.execute("UPDATE items SET rich_text_html='<i>legacy body</i>' WHERE id=?", [id])?;
            let names = conn.prepare("PRAGMA table_info(items)")?
                .query_map([], |row| row.get::<_,String>(1))?.collect::<rusqlite::Result<Vec<_>>>()?;
            assert!(names.contains(&"phrase_title".to_string()));
            Ok(())
        })?;
        let old = db_load_item_full(id).unwrap();
        assert_eq!(old.display_title(), "legacy body");
        db_save_phrase(id, "Saved title", "legacy body").unwrap();
        let summary = db_load_items_page(&query("Saved", 1), None, 200)?.0.remove(0);
        assert_eq!(summary.phrase_title, "Saved title");
        assert_eq!(clip_item_to_summary(&db_load_item_full(id).unwrap()).phrase_title, summary.phrase_title);
        assert_eq!(db_load_item_full(id).unwrap().rich_text_html, old.rich_text_html);
        Ok(())
    })).unwrap();
}

#[test]
fn ordinary_page_query_uses_matching_index_without_temporary_sort() {
    crate::db_runtime::with_test_protected_texts(&[], || crate::db_runtime::with_test_db(|| {
        let mut ids = Vec::new();
        for i in 0..6 { ids.push(insert(&format!("common record {i}"), "", 0)?); }
        with_db(|conn| {
            conn.execute("UPDATE items SET pinned=1 WHERE id IN (?1,?2)", params![ids[0], ids[4]])?;
            for term in ["", "common"] {
                let (sql, values) = items_page_query_sql(&query(term, 0), None, 200);
                let plan = conn.prepare(&format!("EXPLAIN QUERY PLAN {sql}"))?
                    .query_map(params_from_iter(values.iter()), |row| row.get::<_, String>(3))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                assert!(!plan.iter().any(|detail| detail.to_ascii_uppercase().contains("TEMP B-TREE")), "{term}: {plan:?}");
                assert!(plan.iter().any(|detail| detail.contains("idx_items_category_list_order")), "{term}: {plan:?}");
            }
            let legacy_exists: i64 = conn.query_row("SELECT count(*) FROM sqlite_master WHERE type='index' AND name='idx_items_category_pinned_id'", [], |row| row.get(0))?;
            assert_eq!(legacy_exists, 1);
            Ok(())
        })?;
        let expected = vec![ids[4], ids[0], ids[5], ids[3], ids[2], ids[1]];
        let rows = db_load_items_page(&query("common", 0), None, 200)?.0;
        assert_eq!(rows.iter().map(|item| item.id).collect::<Vec<_>>(), expected);
        let (page, cursor, more) = db_load_items_page(&query("common", 0), None, 2)?;
        assert!(more);
        assert_eq!(page.iter().map(|item| item.id).collect::<Vec<_>>(), expected[..2]);
        let following = db_load_items_page(&query("common", 0), cursor, 200)?.0;
        assert_eq!(following.iter().map(|item| item.id).collect::<Vec<_>>(), expected[2..]);
        Ok(())
    })).unwrap();
}

/// Actual production SQL over synthetic on-disk data. Opt in with:
/// ZSCLIP_SEARCH_BENCH_DIR=D:\codex操作目录\search-bench
/// cargo test --release search_phrase_tests::synthetic_search_benchmark -- --ignored --nocapture --test-threads=1
#[test]
#[ignore]
fn synthetic_search_benchmark() {
    let dir = std::path::PathBuf::from(std::env::var_os("ZSCLIP_SEARCH_BENCH_DIR").expect("set ZSCLIP_SEARCH_BENCH_DIR to an operations directory"));
    std::fs::create_dir_all(&dir).unwrap();
    let mut report = vec!["rows,query,returned_rows,first_ms,p50_ms,p95_ms,max_ms".to_string()];
    for count in [200_usize, 1_000, 10_000, 100_000] {
        let db = dir.join(format!("synthetic-{count}-{}.db", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        crate::db_runtime::with_test_protected_texts(&["synthetic-protected-value"], || crate::db_runtime::with_test_db_path(&db, || {
            with_db_mut(|conn| {
                let tx = conn.transaction()?;
                {
                    let mut statement = tx.prepare("INSERT INTO items(category,kind,preview,text_data,rich_text_html,source_app,created_at) VALUES(0,'text',?1,?2,?3,'SyntheticEditor','2026-10-05 12:00:00')")?;
                    for i in 0..count {
                        let preview = format!("合成记录 {i} common");
                        let body = if i % 100 == 0 { format!("{preview} {} 稀有尾词", "长文本内容".repeat(512)) } else { format!("{preview} {}", "常规正文 ".repeat(12)) };
                        let html = (i % 20 == 0).then_some(format!("<b>{body}</b>"));
                        statement.execute(params![preview, body, html])?;
                    }
                }
                tx.commit()
            })?;
            let local_date: String = with_db(|conn| conn.query_row("SELECT date(created_at,'localtime') FROM items ORDER BY id LIMIT 1", [], |row| row.get(0)))?;
            let date_query = format!("日期:{local_date} 稀有尾词");
            let frequent_count = count.min(200);
            let rare_count = count.div_ceil(100).min(200);
            for (term, expected_rows) in [("common", frequent_count), ("稀有尾词", rare_count),
                ("没有这样的条目", 0), ("合", frequent_count), ("应用:syntheticeditor 合成", frequent_count),
                (date_query.as_str(), rare_count)] {
                let mut times = Vec::new();
                let mut returned_rows = 0;
                for iteration in 0..51 {
                    let start = Instant::now();
                    let rows = db_load_items_page(&query(term, 0), None, 200)?.0;
                    times.push(start.elapsed().as_secs_f64() * 1000.0);
                    returned_rows = rows.len();
                    assert_eq!(returned_rows, expected_rows, "rows={count}, query={term}, iteration={iteration}");
                }
                let first = times.remove(0);
                times.sort_by(f64::total_cmp);
                let row = format!("{count},{term},{returned_rows},{first:.3},{:.3},{:.3},{:.3}", times[24], times[47], times[49]);
                println!("{row}"); report.push(row);
            }
            Ok(())
        })).unwrap();
    }
    std::fs::write(dir.join("search-sql-benchmark.csv"), report.join("\n")).unwrap();
}
