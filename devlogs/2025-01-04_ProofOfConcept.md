---
title: Proof of Concept
date: 2025-01-04
author: toshio
tags: 
---

# はじめに

ICP Japanが主催する[ICP Hackathon 2025](https://app.akindo.io/wave-hacks/0nvK8Rd9dfzj63PZV)をきっかけにして、Internet Computer上で個人用途の分散型ストレージの実現を目指します。

## Canister as a personal computer

Internet ComputerのCanisterを『Storage』として利用することを考えます。

既存のAWS S3、Azure Storage、Google Cloud Storageとの大きな違いは、Wasmモジュールによりデータの管理方法やアクセスI/Fを任意に決定できるプログラマブルな点です。

プログラマブルであることにより、ただファイルを保存するという機能だけではなく、DBのような非ファイルのデータも柔軟に取り扱えるようI/Fを追加・変更することもできます。  
また、GitやSubversionのようなバージョン管理機能を有することも可能でしょう。

これから先、数十年、数百年という長期にわたってデータを記録・保管することを考えた場合、特定のベンダーロックインされることなく、また、データの完全性・可用性の観点で、アクセスI/Fやデータ構造などの仕様がオープンに議論されて決定していくことはとても重要であり、分散型のクラウドストレージは透明性が高いと考えます。

## Canister as a personal computer

Internetが普及する以前、多くのPersonal Computerはスタンドアローンで利用され、各アプリケーションのデータはComputer内に完結しており、バックアップとして補助記憶装置に保管されるだけでした。

しかしながら、様々なWebサービスを利用している現在では、自分のデータがWebサービス側にのみ保存されていることも多く、サービスを利用しなくなったり、サービス停止によってデータにアクセス出来なくなってしまうことが多々あります。

本来、自分が持つべきデータをWebサービス側が持っているという現状に対して、Canisterを活用することで解決できるのではないかと考えています。

Webサービスが今までローカルPCにデータを保存しなかった理由として、WebブラウザからPCに自由にアクセスできないセキュリティ上の制限や、どのデバイスからアクセスしてもサービスが利用できるようにする点が挙げられます。

Internet上に自分のComputer（Canister）を配置して適切なアクセス制御を行うことで、サービスプロバイダー側がすべての個人データを持つ必要性も減り、データの主権を取り戻せるようになると考えています。自分でデータを持つようになると維持コストが費用が発生しますが、これまでサービスプロバイダー側は個人情報を利用できる代わりに負担してきたものです。  
これからの時代は、自分の個人データは資産として取り扱い、自身で管理し、自分の同意の下、データを切り売りできるようになるだろうと予想されます。

また、PCやスマホからGoogle DriveやiCloudなどのクラウドストレージに容易にデータにアクセスしたりデータの同期が行われることと同様、様々なデバイスから自分のCanisterへデータアクセスができ、そのCanisterを通じて様々なWebサービスを利用するような仕組みに変わる可能性が考えられます。

## Generation of Personal Storage

| バージョン  | 概要                                                           |
| :---------- | :------------------------------------------------------------- |
| Storage 1.0 | Local (Tape, FDD, HDD, CD-R, ZIP, DVD, Blue-ray, SSD, ...etc.) |
| Storage 2.0 | NAS                                                            |
| Storage 3.0 | クラウドストレージ (Google Drive, OneDrive, iCloud ...etc.)    |
| Storage 4.0 | 分散型クラウドストレージ                                       |

## 提供したい機能

| 機能           | 概要                           |
| :------------- | :----------------------------- |
| データの保管   | 画像、写真、文書などの保存     |
| データの記録   | 削除・改ざんざん防止、真贋証明 |
| データの暗号化 | 大切なデータの暗号化           |
| アクセス制御   | データへのアクセス制御         |
| サービス連携   | Webサービスのデータを管理      |

## 制約事項

### ストレージ最大サイズ

Canisterが記録できる最大のデータ容量は、2025年2月時点で **500GiB** です。

https://internetcomputer.org/docs/current/developer-docs/smart-contracts/maintain/storage#stable-memory

>The maximum storage limit for stable memory is 500GiB if the subnet the canister is deployed on can accommodate it.

### ストレージコスト

1GB/月のデータをCanisterが保存するのに必要なコストは、2025年2月時点で **$0.43** です。

https://internetcomputer.org/docs/current/developer-docs/smart-contracts/maintain/storage#storage-cost

>Storage cost is calculated based on the GiB of storage used by a canister per second, costing 127_000 cycles on a 13-node subnet and 127_000 / 13 * 34 cycles on a subnet with 34 nodes. In USD, this works out to about $0.43 and $1.13, respectively, for storing 1 GiB of data for a 30-day month. The cost is the same whether the canister is using heap memory, stable memory, or both.

### データの秘匿性

Internet Computer上のCanisterは、複数のノード上に配置されて実行されます。  
2025年2月現時点では、Canister内のHeap MemoryやStable Memoryはとくに暗号化されていないため、悪意あるノードプロバイダーがいた場合、内容が閲覧できてしまいます。  
将来的には、SEV-SNPといった仕組みによりハードウェアレベルで暗号化され、ノードプロバイダーからも見えなくなるとのことです。  
本ストレージは、このハードウェアレベルの暗号化が適用されていることを想定した運用とします。

ソフトウェアレベルでのデータ暗号化を実現する場合、エンドツーエンドで[VetKeys](https://internetcomputer.org/blog/features/vetkey-primer)などの暗号技術を利用することも考えられますが、将来に復号できなくなるリスクや、Canister内でのデータ活用を想定した場合にCanister内のプログラムからplainで扱えるのがよいと考えているため、ストレージ機能としてデータ暗号化は行わず、上位アプリケーション層で考慮するものとします。
