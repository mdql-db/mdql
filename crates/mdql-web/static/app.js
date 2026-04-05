// MDQL Client — browser UI

let currentSort = { column: null, asc: true };
let lastResult = null;

// Load tables on startup
document.addEventListener('DOMContentLoaded', () => {
    loadTables();
    loadHistory();

    // Ctrl+Enter to run
    document.getElementById('sql-input').addEventListener('keydown', (e) => {
        if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
            e.preventDefault();
            runQuery();
        }
    });
});

async function loadTables() {
    try {
        const res = await fetch('/api/tables');
        const data = await res.json();
        const list = document.getElementById('table-list');
        list.innerHTML = '';
        for (const table of data.tables) {
            const li = document.createElement('li');
            li.textContent = table.name;
            if (table.row_count !== undefined) {
                const badge = document.createElement('span');
                badge.className = 'row-count';
                badge.textContent = table.row_count;
                li.appendChild(badge);
            }
            li.onclick = () => selectTable(table.name, li);
            list.appendChild(li);
        }
    } catch (err) {
        console.error('Failed to load tables:', err);
    }
}

async function selectTable(name, li) {
    // Update active state
    document.querySelectorAll('#table-list li').forEach(l => l.classList.remove('active'));
    li.classList.add('active');

    // Load schema detail
    try {
        const res = await fetch(`/api/tables/${encodeURIComponent(name)}`);
        const data = await res.json();
        showSchema(data);
    } catch (err) {
        console.error('Failed to load schema:', err);
    }

    // Set default query
    const input = document.getElementById('sql-input');
    if (!input.value.trim()) {
        input.value = `SELECT * FROM ${name} LIMIT 20`;
    }
}

function showSchema(data) {
    const detail = document.getElementById('schema-detail');
    detail.style.display = 'block';
    document.getElementById('schema-table-name').textContent = data.table;

    const fields = document.getElementById('schema-fields');
    fields.innerHTML = '';

    if (data.frontmatter) {
        for (const [name, def] of Object.entries(data.frontmatter)) {
            const div = document.createElement('div');
            div.className = 'field-entry';
            div.innerHTML = `<span class="field-name">${esc(name)}</span><span class="field-type">${esc(def.type)}</span>${def.required ? '<span class="field-required">*</span>' : ''}`;
            fields.appendChild(div);
        }
    }

    if (data.sections) {
        for (const [name, def] of Object.entries(data.sections)) {
            const div = document.createElement('div');
            div.className = 'field-entry';
            div.innerHTML = `<span class="field-name">${esc(name)}</span><span class="field-type">section</span>${def.required ? '<span class="field-required">*</span>' : ''}`;
            fields.appendChild(div);
        }
    }
}

async function runQuery() {
    const sql = document.getElementById('sql-input').value.trim();
    if (!sql) return;

    const format = document.getElementById('format-select').value;
    const status = document.getElementById('status-text');
    const results = document.getElementById('results-area');

    status.textContent = 'Running...';
    status.style.color = 'var(--text-dim)';

    const start = performance.now();

    try {
        const res = await fetch('/api/query', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ sql, format }),
        });
        const data = await res.json();
        const elapsed = ((performance.now() - start) / 1000).toFixed(3);

        if (data.error) {
            results.innerHTML = `<div class="error-message">${esc(data.error)}</div>`;
            status.textContent = 'Error';
            status.style.color = 'var(--red)';
        } else if (format === 'json' || format === 'csv') {
            results.innerHTML = `<pre class="results-text">${esc(data.output)}</pre>`;
            status.textContent = `${elapsed}s`;
            status.style.color = 'var(--green)';
        } else {
            lastResult = data;
            currentSort = { column: null, asc: true };
            renderTable(data.columns, data.rows);
            const rowCount = data.rows ? data.rows.length : 0;
            status.textContent = `${rowCount} row${rowCount !== 1 ? 's' : ''} · ${elapsed}s`;
            status.style.color = 'var(--green)';
        }

        addHistory(sql);
    } catch (err) {
        results.innerHTML = `<div class="error-message">Connection error: ${esc(err.message)}</div>`;
        status.textContent = 'Error';
        status.style.color = 'var(--red)';
    }
}

function renderTable(columns, rows) {
    if (!columns || !rows || rows.length === 0) {
        document.getElementById('results-area').innerHTML = '<div class="placeholder">No results</div>';
        return;
    }

    let html = '<table class="results"><thead><tr>';
    for (const col of columns) {
        let cls = '';
        if (currentSort.column === col) {
            cls = currentSort.asc ? 'sort-asc' : 'sort-desc';
        }
        html += `<th class="${cls}" onclick="sortBy('${esc(col)}')">${esc(col)}</th>`;
    }
    html += '</tr></thead><tbody>';

    for (const row of rows) {
        html += '<tr>';
        for (const col of columns) {
            const val = row[col];
            const display = val === null || val === undefined ? '' : String(val);
            html += `<td title="${esc(display)}">${esc(truncate(display, 100))}</td>`;
        }
        html += '</tr>';
    }

    html += '</tbody></table>';
    document.getElementById('results-area').innerHTML = html;
}

function sortBy(column) {
    if (!lastResult) return;

    if (currentSort.column === column) {
        currentSort.asc = !currentSort.asc;
    } else {
        currentSort.column = column;
        currentSort.asc = true;
    }

    const rows = [...lastResult.rows];
    rows.sort((a, b) => {
        const va = a[column], vb = b[column];
        if (va === null || va === undefined) return 1;
        if (vb === null || vb === undefined) return -1;
        if (typeof va === 'number' && typeof vb === 'number') {
            return currentSort.asc ? va - vb : vb - va;
        }
        const sa = String(va), sb = String(vb);
        const cmp = sa.localeCompare(sb);
        return currentSort.asc ? cmp : -cmp;
    });

    renderTable(lastResult.columns, rows);
}

// History management (localStorage)
const HISTORY_KEY = 'mdql_history';
const MAX_HISTORY = 50;

function addHistory(sql) {
    let history = JSON.parse(localStorage.getItem(HISTORY_KEY) || '[]');
    history = history.filter(h => h !== sql);
    history.unshift(sql);
    if (history.length > MAX_HISTORY) history = history.slice(0, MAX_HISTORY);
    localStorage.setItem(HISTORY_KEY, JSON.stringify(history));
    renderHistory(history);
}

function loadHistory() {
    const history = JSON.parse(localStorage.getItem(HISTORY_KEY) || '[]');
    renderHistory(history);
}

function renderHistory(history) {
    const list = document.getElementById('history-list');
    list.innerHTML = '';
    for (const sql of history.slice(0, 20)) {
        const li = document.createElement('li');
        li.textContent = sql;
        li.title = sql;
        li.onclick = () => {
            document.getElementById('sql-input').value = sql;
            document.getElementById('sql-input').focus();
        };
        list.appendChild(li);
    }
}

function esc(str) {
    if (str === null || str === undefined) return '';
    return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;');
}

function truncate(str, max) {
    if (str.length <= max) return str;
    return str.substring(0, max) + '…';
}
