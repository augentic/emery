# Design

## Overview

Sign-in issues a session; orders are placed and cancelled under it.

## Domain model

Type: orders.order
```
interface Order { id: string }
```

Type: orders.line
```
interface Line { sku: string }
```
