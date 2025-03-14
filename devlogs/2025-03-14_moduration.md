---
title: crate
date: 2025-03-14
author: toshio
tags: 
---

Crateを取り込んだらI/Fを簡単に追加できるようにしたい。

以下を見るとexport-candidを使う場合、crate側ではエンドポイントを登録せず、利用者側で#[query]や#[update]のI/Fを改めて用意するとよいとのこと。

https://forum.dfinity.org/t/export-candid-from-crate-used-by-consumer/26455

現在のcanistorageをcrateとして提要すると、利用する側でビルドエラーとなる。  
crateとして用意するのではなく、wasmモジュールとして用意する方向で検討が必要。

将来的に備えてcrate名は確保しておいたものの、今後利用できるようになるかは微妙。

https://crates.io/crates/canistorage/
