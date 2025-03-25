# Canistorage

## Concept

Canistorageは、Internet Computerの仕組みを利用した分散型のクラウドストレージです。  
まだ開発の初期段階です。

Internet Computer上でユーザーやアプリケーションを識別するための『Principal』ベースでアクセス制御を行い、
個人や家族（または企業・部署等）の単位でCanister (→Canistorage) を用意して、データの記録・保管していくことを目指します。

これから先、数十年、数百年という長期にわたってデータを管理していくことを考えた場合に、ベンダーロックインされることなく、
また、データの完全性・可用性の観点で、アクセスI/Fやデータ構造などの仕様がオープンに議論されて決定していくことは重要です。  
Canistorageは、そのような観点から、オープンな仕様でデータを管理することを目指しています。

Pricipalごとにディレクトリやファイルのアクセス制御が可能なため、使用するDAppごとに異なるPrincipalを用いることによって、  
（つまり、Internet IdentityでDAppごとに異なるPrincipalが割り当てられる仕組みを利用することによ）、DAppごとにアクセス制御を実現できます。

また、今後、様々なCanisterが自律分散型で動作していくという世界線で、自分のデータがどこにあり（→Canistorage内に集約され）、どのCanisterがどのデータに対して読取権限があるのか、Principalベースでのアクセス制御できる分散型のクラウドストレージは必要となるでしょう。

本来、Canister自体がWASIに対応しPrincipalベースのアクセス制御を有するファイルシステム機能を持っていることが望ましいと個人的には考えており、  
このPoCは、あくまでもCanisterにまともなファイルシステム機能が実装されるまでの過渡的な実装にすぎません。

将来的には、単なるファイルの管理機能だけではなく、DBなど非ファイルデータやバージョン管理機能、改ざん抑止、暗号化、署名、タイムロックなど、より高度なデータ管理機能を、オープンなI/F仕様を検討し取り込んでいければと思います。

## マイルストーン

| バージョン | 概要                                     |
| :--------- | :--------------------------------------- |
| 0.1.0      | PoC 基本的なファイル操作と権限制御を実装 |

## Canistorage公開インターフェース

Canistorage v0.1.0では、以下の公開メソッドを提供しています。

| メソッド名                                                    | 種別   | 概要                                                | 備考                                |
| :------------------------------------------------------------ | :----- | :-------------------------------------------------- | :---------------------------------- |
| version                                                       | query  | バージョン情報を返す                                |                                     |
| initCanistorage                                               | update | Canistorageの初期設定を行う                         | 呼び出したPrincipalがRoot権限を持つ |
| listFiles                                                     | query  | 指定ディレクトリのファイル/ディレクトリ一覧を返す   |                                     |
| getInfo                                                       | query  | 指定ディレクトリ／ファイルの情報を返す              |                                     |
| createDirectory                                               | update | ディレクトリを作成する                              |                                     |
| deleteDirectory                                               | update | ディレクトリを削除する                              |                                     |
| save                                                          | update | ファイルを保存する (小サイズのファイル)             |                                     |
| beginUpload,<br/>sendData,<br/>commitUpload,<br/>cancelUpload | update | ファイルを保存する  大きいサイズのファイル）        |                                     |
| load                                                          | query  | ファイルを取得する (小サイズのファイル)             | 大きいサイズの取得は仕様検討中      |
| delete                                                        | update | ファイルを削除する                                  |                                     |
| hasPermission                                                 | query  | ディレクトリに対する呼び出し元のアクセス権限を返す  |                                     |
| addPermission                                                 | update | ディレクトリ/ファイルに対してアクセス権限を付与する |                                     |
| removePermission                                              | update | ディレクトリ/ファイルからアクセス権限をはく奪する   |                                     |
| （getAllInfoForPoC）                                          | query  | ディレクトリ／ファイル情報一括取得                  | PoC用に一時作成                     |
| （forceResetForPoC）                                          | update | Canistorageの内容を強制リセット                     | PoC用に一時作成                     |

## Canistorage動作検証用テストサイト

https://eqncz-yyaaa-aaaal-qsliq-cai.icp0.io/

アクセス対象のCanistorageのCanister Idは、`tw3il-viaaa-aaaak-quepa-cai`です。

## 仕様に関する補足説明・制限・課題

### 独自性 （Asset Canisterとの違い）

Canisterにファイルを保存する仕組みとして、Asset Canisterがあります。Asset Canisterの大きな目的は、Webアプリケーションのリソースを保管しHTTPS経由で取得できるようにすることでFrontendアプリケーションをホストすることです。そのため、データに対するアクセス制御という点はあまり考慮されていません。

Canistorageは、商用クラウドサービスの[Amazon S3](https://aws.amazon.com/s3/)や[Azure Blob Storage](https://learn.microsoft.com/en-us/azure/storage/blobs/storage-blobs-overview)のようなストレージを、Internet Computer上に、Principalベースでアクセス制御できる分散型クラウドストレージを目指しています。

また、個人用ストレージという分野では、[Daniel](https://x.com/realdanmccoy)氏による[uBin](https://h3cjw-syaaa-aaaam-qbbia-cai.ic0.app/)という素晴らしいアプリケーションがありますが、以下のフォーラムから仕様を読むかぎり、現時点ではまだ競合するものでは無いように思っています。

https://forum.dfinity.org/t/ubin-formerly-asset-app/32985

Canistorageは、様々なユーザーやDApp、Canisterからのアクセスされることを想定しPrincipalベースのアクセス制御を行うストレージで、Canistorage単体としてファイルビューワーを提供するものではありません。

『Webサービス側が個人のデータを所有する』という従来のWebサービスの仕組みを、『個人のデータは本人が所有し、Webサービス側に対して必要に応じてアクセス権限を付与する』という個人のデータ主権を取り戻す仕組みへと変えていくことを目指しています。

### アクセス制御

Canistorage v0.1.0では、以下のアクセス制御を用意しています。

| アクセス権限の種類 | 概要                                                                           |
| :----------------- | :----------------------------------------------------------------------------- |
| managable          | 指定ディレクトリ配下、または指定ファイルに対するアクセス権限を付与、剥奪できる |
| readable           | 指定ディレクトリ配下、または指定ファイルに対して読み取りができる               |
| writable           | 指定ディレクトリ配下、または指定ファイルに対して書き込みができる               |

アクセス権限について、仕様の整理がまだ不十分なので引き続き検討が必要です。

- 権限は下位ディレクトリに継承される仕様としているため、配下にある一部ディレクトリに対してアクセス拒否する仕組みがない。
- ファイル一覧取得の権限、ディレクトリ内のファイル作成・削除の権限、ファイル書き込み権限の分離
- 自身に対する権限の制御 （自分自身に対してmanagable権限の剥奪は可？不可？）
- グループ権限の仕組みの検討

### ディレクトリ/ファイルのメタ情報

Principalベースのファイルシステムの実現のために、[`ic-wasi-polyfill`](https://github.com/wasm-forge/ic-wasi-polyfill)の[`stable-fs`](https://github.com/wasm-forge/stable-fs)の仕組みを利用しています。

各ディレクトリやファイルに対するアクセス権限などを管理するメタ情報は、ファイルシステムの一部としてLinuxの[inode](https://ja.wikipedia.org/wiki/Inode)のような形で実現されているものが望ましいのですが、一から新しいファイルシステムを設計することは難しいため、`ic-wasi-polyfill`を利用し、（非効率ではあるものの）メタ情報をファイルの形式で、1ファイルにつき1ファイルに対応づけて管理しています。

暫定的に、以下のようにファイル名の先頭に、（きっとあまり使わないだろう）バッククォート(``  ` ``)を付与したファイルにメタ情報を格納しています。

| 項目     | ファイル名                           | 備考 |
| :------- | :----------------------------------- | :--- |
| ファイル | `<fileName>`                         |      |
| メタ情報 | `` `<fileName>`` | Leading backquote |      |

本来、ファイルシステムはCanister側の仕組みとして一から設計されCanisterの基本機能として提供されていることが望ましいと個人的には考えており、公式が対応するまでの暫定的な仕組みです。

## ビルド & ローカル実行

Canistorageをローカルの実行環境にデプロイして動作させる手順を示します。

### Setup

```bash
$ cargo install wasi2ic
$ cargo install candid-extractor
$ rustup target add wasm32-wasip1
```

### Clone canistorage project from Github

```bash
$ git clone https://github.com/toshio/canistorage.git
$ cd canistorage
```

### Start Local Canister Runtime Environtment

```
$ dfx start --clean --background
```

### Build & Deploy

```bash
$ dfx deploy
```

## 初期設定

Canisterを新規デプロイした時点では、ルートディレクトリ（/）のみ存在し、デプロイしたユーザーのPrincipalによるアクセス権限（managable, readable, writable）が付与されています。

このままでは、dfxコマンド経由でしかCanistorageのファイルにアクセスできないため、ディレクトリを作成したり、特定のPrincipalに対するアクセス許可を設定します。

#### バージョン確認

```bash
$ dfx canister call canistorage version
("canistorage 0.1.0")
```

### 初期化処理

Canistorageを最初に使用する際に呼び出します。呼び出したユーザーのPrincipalがRoot権限を持ちます。

```bash
$ dfx canister call canistorage initCanistorage
```

#### ディレクトリ作成 (例)

```
$ dfx canister call canistorage createDirectory '("/temp")'
```

#### パーミッション付与 (例)

```
$ dfx canister call canistorage addPermission '("/temp", principal "2vxsx-fae", false, true, true)'
```

### 指定ディレクトリの情報を取得 (例)

```
$ dfx canister call canistorage getInfo '("/")'
```
