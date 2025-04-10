---
title: ICP Hackerthon 2025 Wave4
date: 2025-03-01
author: toshio
tags: 
---

## Why Storage?

データを保管するストレージとして、Internet Computer上でユーザーやCanisterを識別する『Principal』ベースでのアクセス制御ができるストレージがあるとよい。

まずは基本的なファイル管理の機能を提供することを目指し、少しずつ機能強化をしていく。

## Interface

ICPのI/Fにおいて、一度にやりとりできるメッセージ長は約2MB未満であるため、アップロード／ダウンロードするファイルが約2MB超かどうかで方式を変える必要がある。

## Canister

データをどのように保存するかを考えた場合、まずは既存のディスクと同じようなファイル形式にするのがよいかと考えた。  
一からCanister向けのファイルシステムをつくるのは工数的にも非常に難しいため、[ic-wasi-polyfill](https://github.com/wasm-forge/ic-wasi-polyfill)ベースでファイル管理できるようにする。

Dfinity公式でAsset Canisterがあるが、Stable Memoryに保存しているわけではなく、Frontend用に用意されたWeb向けの仕組みであり、今後の拡張性を考えて独自に実装することとする。

本来はCanister標準機能として、ファイルシステム機能が用意されていることが望ましいと考える。
