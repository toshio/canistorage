---
title: File Info
date: 2025-03-03
author: toshio
tags: 
---

```mermaid
flowchart TB
  node_1(["save()"])
  node_2["validate_path()"]
  node_3["get_file_info()"]
  node_4["check_write_permission()"]
  node_1 --> node_2
  node_2 --> node_3
  node_3 --> node_4
```
