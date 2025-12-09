# CLAUDE.md

このファイルはClaude Code (claude.ai/code) がこのリポジトリで作業する際のガイダンスを提供します。

## プロジェクト概要

WebGPUベースのゲームエンジン。バックエンドはRust（wgpu）で実装しWASMにコンパイル、エディタUIはReact/TypeScriptで構築。レンダリング（Rust）とUI（React）を分離し、API経由で通信するアーキテクチャ。

## ビルドコマンド

```bash
# WASMモジュールをビルド（出力先: editor/src/wasm/）
pnpm build:wasm

# エディタ開発サーバー起動（Vite）
pnpm dev:editor

# WASMビルド後にエディタ開発サーバー起動
pnpm dev

# Rust変更を監視して自動でWASMリビルド
pnpm watch:rust

# Rustテスト実行
cargo test --workspace

# Rustリント
cargo clippy --workspace

# TypeScriptリント
cd editor && pnpm lint
```

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────┐
│  Editor UI (React + TypeScript)  -  editor/src/         │
│  - コンポーネント: Viewport, Hierarchy, Inspector       │
│  - Engine APIラッパーとReact hooks                      │
└─────────────────────────────────────────────────────────┘
                          │ wasm-bindgen
                          ▼
┌─────────────────────────────────────────────────────────┐
│  Rust Engine (WASM)  -  crates/                         │
│  ┌─────────────────┐ ┌─────────────────┐                │
│  │  engine-core    │ │  engine-renderer│                │
│  │  - ECSシステム  │ │  - wgpu設定     │                │
│  │  - World, Entity│ │  - パイプライン │                │
│  │  - Components   │ │  - Mesh, Camera │                │
│  └─────────────────┘ └─────────────────┘                │
│  ┌─────────────────────────────────────┐                │
│  │  engine-wasm                        │                │
│  │  - WASMバインディング (wasm-bindgen)│                │
│  │  - JS <-> Rust ブリッジ             │                │
│  └─────────────────────────────────────┘                │
└─────────────────────────────────────────────────────────┘
```

### クレート責務

- **engine-core**: ECS実装（Entity, Component, World）、数学型（glam使用）、Transform/Nameコンポーネント
- **engine-renderer**: wgpuレンダリング、サーフェス、パイプライン、メッシュ/テクスチャ管理
- **engine-wasm**: WASMエントリポイント、wasm-bindgenエクスポート、JS連携レイヤー

### 使用技術

- **Rust**: wgpu 27（WASM用WebGLバックエンド）、glam（数学）、bytemuck（GPUデータ）、serde（シリアライズ）
- **TypeScript**: React 19、Vite 7、pnpmパッケージマネージャー
- **WASM**: wasm-pack（`--target web`）、wasm-bindgen（JSバインディング）

## 開発ワークフロー

1. `crates/`でRustコードを変更
2. `pnpm build:wasm`でWASMにコンパイル
3. WASM出力は`editor/src/wasm/`に配置
4. エディタは`import init, { ... } from './wasm/engine_wasm'`でWASMモジュールをインポート

反復開発には、ターミナル1で`pnpm watch:rust`、ターミナル2で`pnpm dev:editor`を実行。

## 設計原則

- **API駆動**: Entity操作はすべてWASM APIレイヤー経由
- **クエリベース**: SQLライクなクエリでEntityを柔軟に検索・操作
- **型安全**: TypeScript/Rustの型システムを最大限活用

## 開発フェーズ

現在Phase 1（最小ECSコア）を実装中。タスク進捗は [.claude/phase1/tasks/README.md](.claude/phase1/tasks/README.md) を参照。

```
Phase 0: 技術検証 ✅ (三角形描画完了)
Phase 1: 最小ECSコア 🔄
Phase 2: クエリシステム
Phase 3: エディタ基盤
Phase 4: 機能拡充
```

## 関連ドキュメント

- [設計書](.claude/game-engine-design.md) - 全体設計、API仕様、開発フェーズ詳細
- [Phase 1 タスク](.claude/phase1/tasks/README.md) - 現在のタスク一覧と依存関係
