# Serena / Context7 起動方法

## 前提
- Python 3.11
- uvを導入
- Port 32123を使用

## Serena の起動

### 既存のプロセス停止
```bash
    pkill -f serena-mcp-server
```

### Serena初期設定
#### venv作成
```bash
    uv venv
```
#### uv環境の有効化
```bash
    source .venv/bin/activate
```
プロンプトの先頭に (venv) などが付けばOK：
(venv) uka@uka-ThinkPad:~/Desktop/PJ_rikai$

#### Serenaをインストール
```bash
    uv pip install "git+https://github.com/oraios/serena#egg=serena"

```

### Serena起動
```bash
    serena-mcp-server --port 32123

```

### 動作確認
```bash
    ss -ltnp | grep 32123
```
→ LISTEN 表示で正常

## Context7 の起動

### 既存のプロセス停止
```bash
    pkill -f context7-mcp
```

### 起動
```bash
    npx --yes @upstash/context7-mcp
```

### 動作確認
```bash
    pgrep -af context7-mcp 
```
→ node context7-mcp 表示で正常

## Codex から接続確認
/mcpで表示


