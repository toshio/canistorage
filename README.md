# Canistorage

## Concept

Internet上にオープン仕様の分散型ストレージを開発するプロジェクトです。

特定のクラウドベンダーには依存しない分散型クラウド環境であるInternet Computer上に、オープンな仕様の分散型ストレージを **1 Canister/人** という単位で用意し、個人のデータをただファイルとして保存するという目的だけにとどまらず、様々なWebサービスとも連携して、個人のデータ主権を取り戻せる仕組みづくりを考えていきたいです。

## 提供機能

## 公開API

| メソッド名 | 種別  | 概要                 |
| :--------- | :---- | :------------------- |
| version    | query | バージョン情報を返す |

## マイルストーン

| バージョン | 概要                                     |
| :--------- | :--------------------------------------- |
| 0.1.0      | PoC 基本的なファイル操作と権限制御を実装 |

## ビルド & ローカル実行

### Setup

```bash
$ cargo install wasi2ic
$ cargo install candid-extractor
$ rustup target add wasm32-wasip1
```

### Start Local Canister Runtime Environtment

```
$ dfx start --clean --background --pocketic
```

### Build & Deploy

```bash
$ dfx deploy
```

###

```bash
$ dfx canister call canistorage version
("canistorage 0.0.0")
```



###

```bash
$ dfx canister call canistorage createDirectory '("/dir1")'
$ dfx canister call canistorage save '("/file", "text/plain", vec {31; 32; 33}, true)'
```

$ dfx canister call canistorage add_permission '(principal "2vxsx-fae", "/", true, true)'