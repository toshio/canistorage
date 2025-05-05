---
title: canistorage-jsr
date: 2025-05-05
author: toshio
tags: 
---

Canistorageは、分散型クラウドであるInternet Computer上のCanisterをストレージとして利用することを目標に開発しています。

ディレクトリごとにPrincipalベースのアクセス制御ができる仕組みのため、特定のECDSA秘密鍵を持っているユーザーやアプリケーションのみがアクセス可能なディレクトリを用意したり、Internet Identityが同じユーザーに対しDappsごとに異なるPrincipalを割り当てる特徴を利用して、個人のデータに対するアクセス制御が可能です。

Canistorageの公開インタフェースは、ICPの[candid](https://internetcomputer.org/docs/building-apps/interact-with-canisters/candid/candid-concepts)と呼ばれるインターフェース記述言語（IDL）で規定されており、JavaScript、Rust、Java、Pythonnなどプログラミング言語に依存することなくアクセスすることができます。

とはいえ、サイズが数MB以上のファイルの場は複数シーケンスの呼び出しが必要となる等、公開インタフェースをそのまま利用するのは使い勝手が良くないため、より使いやすい形にwrapしたライブラリが用意されていることが望ましいと考えています。

近年はWebアプリケーションをはじめTypeScriptを利用するケースも増えていますので、Node.jsやDenoから利用できるJSRライブラリも用意していく予定です。

### JSRライブラリ

https://jsr.io/@toshio/canistorage

### ソースコード

https://github.com/toshio/canistorage-jsr
