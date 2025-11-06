# Lume 语言设计草案（v0.3）

## 综述

Lume 是一个**强类型、内存安全、多范式**的编程语言，目标是融合现代语言的易用性与系统语言的性能和安全性。~~实际上是 Rust++~~

### 设计哲学

- 显式而非隐式
- 配置而非约定，但提供约定的默认配置
- 关键字而非符号魔法
- 多范式融合，职责分明
- 错误是流状态而非值
- 能看懂很重要

- Explicit, not implicit
- Configurable, not convention-driven - but with sensible defaults
- Keywords over symbolic magic
- Multi-paradigm with clear responsibilities
- Errors are flow states, not values
- Readability matters

### 核心特性

- **语法风格**：类 Dart / TypeScript，使用四空格缩进（非强制）
- **模块系统**：ECMAScript Module（`import` / `export`）
- **内存模型**：类似 Rust 的所有权与生命周期机制，**无垃圾回收（GC-free）**
- **类型哲学**：
  - Everything is Object（所有值皆为对象），但对基本类型做底层优化
  - `class` 仅用于继承与数据封装，**多态由 `trait` 实现**
  - `Error` `None` 为**状态**而非**值**
- **执行方式**：支持编译（AOT）与解释（REPL/JIT）两种模式

## 设计细节

仅列出与 Rust 的差异，未列出但存在于 Rust 的特性大概率都会实现。

### 语法

- 类 TypeScript 语法
- 强制使用分号和代码块
- 四空格缩进（建议，非语法强制）

### 类型系统

#### 一切皆对象

- 所有值在语法上表现为对象（Everything is Object）
- 支持方法调用语法糖
- **基本类型底层仍为类型**

```text
1.toString()   // 语法糖，底层实现类似 String::from(1)
(true).and(false) // 仅示例；实际 Bool 无 and 方法
```

#### 内置类型别名

| 实际类型 | 别名     |
| -------- | -------- |
| `bool`   | -        |
| `i8`     | -        |
| `u8`     | `byte`   |
| `i16`    | `short`  |
| `u16`    | `ushort` |
| `i32`    | `int`    |
| `u32`    | `uint`   |
| `i64`    | `long`   |
| `u64`    | `ulong`  |
| `f32`    | `float`  |
| `f64`    | `double` |

#### 高级内置类型（部分需引入）

- `Array<T, N>`：固定大小数组，类似 C++ `std::array`
- `List<T>`：动态数组，类似 C++ `std::vector`
- `String`：底层使用 `List<uint32>`，兼容 Unicode
- `Map<K, V>`、`Set<T>`、`Function` 等
- `Self`：表示当前类的特殊类型
- `Optional<T>`：表示可选值。实际类型声明时建议使用 `?` 语法糖
- `Result<T, E>`：表示可能抛出错误的结果。实际建议使用 `throws` 语法糖
- `None`：`Optional<T>` 的一种**状态**，**不是类型**。
- `Error`：错误状态的标签。**语法上非值**。可被“继承”以自定义错误类型

### 所有权系统

- 必须使用 `own` 关键字显式转移所有权
- 非显式转移行为**默认复制（Copy）**（通过 `Copy` trait），未实现则编译时报错
- 返回值自动转移所有权
- 拥有所有权的纯右值自动转移所有权

> own 关键字作用于变量，表示 “交出变量的所有权”

```text
let a = 1;  // ✅正确，1 （i32）可 Copy
let b = "hello";  // ✅正确，字面量 StringSlice 被 Copy
let c = a;  // ✅正确，此时 a、c 都有效

let d = "hello".toString();  // 生成 String
let e = d;  // ❌错误，无法复制 String，需要显式转移所有权
let e = own b;  // ✅正确，显示转移 b 的所有权，b 失效

func foo(a: Bar) -> Bar  // 返回值不需要写 own
{
    return a;  // 返回值自动转移
};

let f = Bar();
let g = foo(own f);  // 调用时显式标注所有权传递，同时纯右值自动转移
```

### 生命周期

- 使用 `'a`、`'b` 等标记生命周期
- 生命周期标注在**类型**上，如 `&'a T`
- 使用 `outlives` 约束生命周期长短关系

```text
func maxRef<'short, 'long>(x: &'short Foo, y: &'long Foo) -> &'long Foo
where
    'long outlives 'short
{
    if x.a > y.a => return x;
    else => return y;
}

class 'a Bar
{
    value: &'a int;
}
```

### 模块系统

Lume 的模块系统建立在三级抽象模型之上：workspace（工作区）、package（包）和 module（模块）。这三级共同构成依赖解析、代码组织与构建分发的基础。

#### 1. 三级模型

##### Workspace（工作区）

工作区由项目根目录下的 `lume.workspace.toml` 文件定义，用于管理多个本地开发的包。工作区本身不参与构建或分发，仅在开发阶段提供跨包引用能力。

```toml
# lume.workspace.toml
members = [
  "crates/core",
  "crates/net",
  "examples/demo"
]
```

成员路径必须指向包含 `lume.toml` 的目录。

##### Package（包）

包是 Lume 中依赖管理、构建和分发的基本单位，由 `lume.toml` 文件定义。每个包具有全局唯一标识，格式为 `[@scope/]name`（例如 `@acme/http` 或 `serde`）。

包默认从 `src/` 目录加载源码，可通过 `src-dir` 配置项覆盖。包可声明依赖、导出映射、链接策略等元数据。

```toml
# lume.toml
name = "http"
scope = "acme"          # 可选，默认无作用域
version = "0.1.0"

src-dir = "source"      # 可选，默认 "src"

[dependencies]
"serde" = "^1.0.0"
```

导入路径的第一段始终对应一个包标识。

##### Module（模块）

模块是包内部的逻辑代码单元，不具备独立身份。其物理形式为：

- 单个 `.l` 文件（如 `src/utils.l`），或
- 包含 mod.l 的目录（如 `src/net/mod.l` 定义 `net` 模块）。

模块路径相对于包的源码根目录，通过子路径引用（例如 `@self/net` 对应 `src/net/mod.l`）。模块不能声明独立依赖或版本，所有依赖由所属包统一管理。

同一目录下禁止同时存在 `name.l` 文件与 `name/` 目录，否则编译报错，以避免路径歧义。

#### 2. 配置文件命名

| 文件                  | 位置             | 作用                         |
| --------------------- | ---------------- | ---------------------------- |
| `lume.workspace.toml` | 项目根目录       | 定义工作区及成员包           |
| `lume.toml`           | 包根目录         | 定义包元数据、依赖与构建配置 |
| `lume.module.toml`    | 模块目录（可选） | 定义模块内部路径映射         |

`lume.module.toml` 优先级高于 `mod.l`，但不能用于规避文件与目录同名冲突。物理布局必须保持无歧义。

#### 3. 路径与导入语义

Lume 严格区分相对路径与包路径，二者语义互斥。

##### 路径分类

- **相对路径**：以 `./` 或 `../` 开头，用于引用当前包内的本地文件或目录。
- **包路径**：不以 `./` 或 `../` 开头，必须解析为某个已声明的包。

所有非相对路径必须对应一个有效的包标识。编译器不会在当前包的源码目录中查找裸包名（例如 `net` 不会匹配 `src/net/`）。

保留作用域 `@self` 自动解析为当前包名，用于安全地引用自身模块。

##### 包路径解析流程

对 `import ... from "@scope/name/sub/path";` 的解析步骤如下：

1. 提取包标识 `@scope/name`；
2. 定位包：
   - 若为 `@self`，则使用当前包；
   - 否则依次检查工作区成员、注册表依赖和内置包（如 `@std`）；
3. 在目标包的源码根目录下解析子路径 `sub/path`：
   - 若路径以 `.l` 结尾，只会尝试 `sub/path.l`
   - 若路径不以 `.l` 结尾：
     - 优先读取 `sub/path/lume.module.toml` 中的 `exports` 映射；
     - 否则尝试 `sub/path/mod.l`；
     - 若路径不以 `/` 结尾，也可尝试 `sub/path.l`；
4. 若未找到，编译报错。

##### 使用建议

- 同一逻辑模块内的文件互引应使用相对路径（如 `./helper.l`）；
- 同包内不同模块之间引用推荐使用 `@self/module`；
- 外部依赖使用完整包路径（如 `@acme/utils`）；
- 测试和示例代码应通过 `@self` 模拟外部用户视角；
- 尽量不要在同一逻辑模块内通过 `@self/...` 引用自身文件。

#### 4. 高级特性

##### 精细导出可见性控制

支持多级可见性修饰：

```text
export func global() {}               // 全局可见（等价于 pub）
export(package) func pkg_only() {}    // 仅当前包内可见（等价于 pub(crate)）
export(parent) func parent_only() {}  // 仅父包及其子包可见（等价于 pub(super)）
export("@self/utils/helpers.l") func local() {}  // 仅指定路径模块可见（等价于 pub(in path)），解析方式同 import
```

路径不支持通配符或目录范围。

##### 模块聚合与路径映射

通过 lume.module.toml 声明导出映射：

```toml
# src/mylib/lume.module.toml
[exports]
"/"            = "lib.l"                # 根路径 → lib.l
"json"         = "formats/json.l"       # mylib/json → formats/json.l
"crypto/aes"   = "crypto/aes_impl.l"    # 支持嵌套路径
```

未在 exports 中声明的文件不可被外部导入。

##### 默认与具名导入导出

- 默认导出写在大括号外：`import App from "pkg";`;
- 具名导出写在大括号内：`import { func } from "pkg";`;
- 不允许 `import { default as X }`，应写作 `import X from "pkg";`。

每个模块最多一个默认导出。

##### 重导出

支持多种重导出形式：

```text
// 将默认导出转为具名导出
export App, { myFunc } from "./mylib";

// 透传默认导出 + 转发具名导出
export default, { myFunc } from "./mylib";

// 重命名具名导出
export { myFunc as myFunc2 } from "./mylib";

// 将具名导出提升为默认导出
export { myFunc as default } from "./mylib";

// 透传所有具名导出，但不透传默认导出
export { * } from "./mylib";

// 透传所有具名导出，并透传默认导出
export default, { * } from "./mylib";
```

所有重导出均为符号转发，无运行时开销。

##### 导入到命名空间

```text
// 仅具名导出
import { * } as mylib from "mylib";

// 具名 + 默认导出（重命名为 App）
import { *, default as App } as mylib from "mylib";

// 部分导入到命名空间
import { myFunc, myOtherFunc, default as App } as mylib from "mylib";
```

若不绑定到命名空间，则禁止显式 `default as X`。

##### `impl` 的导入

```text
// 导入某个 impl
import { impl SomeTrait for SomeType } from "mylib";

// 导入所有导出的 impl
import { impl } from "mylib";

// 导入某 Trait 的所有 impl
import { impl SomeTrait } from "mylib";

// 导入某 Type 的所有 impl
import { impl for SomeType } from "mylib";
```

可通过标记阻止自动导入：

```text
@noAutoImport
export impl SomeTrait for SomeType {}  // 不参与 import { impl }，但可显式导入
```

##### 导入排除

在批量导入时可排除特定项：

```text
// 排除具名导出
import { *, excluding SomeType } from "mylib";

// 排除特定 impl
import { impl for SomeType, excluding impl SomeTrait } from "mylib";

// 多项排除（需用大括号）
import { impl, excluding { impl SomeTrait for SomeType, impl for SomeOtherType } } from "mylib";
```

excluding 后可接单个项或大括号列表。排除不存在项仅触发 lint 警告。

#### 5. 补充规则

- 禁止任何模块间的循环依赖，包括间接循环；
- 文件系统路径比较进行归一化处理，预防大小写或 Unicode 差异导致的跨平台问题；
- Linter 默认警告硬编码本包名，推荐使用 `@self`。

### 注释与命名规范

- 行注释：`//`
- 块注释：`/* */`，不建议行首 `*`
- 文档注释：`///` 或 `/** */`，其中 `/** */` 不建议行首 `*`
- 命名约定：
  - 变量/函数：`camelCase`
  - 类/接口/类型：`PascalCase`
  - 静态常量：`SCREAMING_SNAKE_CASE`
- 允许 Unicode 标识符

### 变量与常量

- `let`：默认不可变
- `late`：延迟初始化的不可变变量。编译器静态分析路径，使用前必须初始化
  - 如果允许不初始化，应当用 `let mut Optional` 类型和 `on None` 处理未初始化情况
- `let mut`：可变变量
- `const`：编译期确定的常量
- 无 `static`，全局常量通过模块导出，需要满足 `'static` 生命周期

### 懒初始化

使用 `@lazy` 宏，通过可调用类型初始化，`value` 解包：

```text
@lazy
let foo: int = [] => someHeavyInit();  // 本例为闭包，也可以是函数等
foo.value  // 解包
```

### 闭包

语法：`[参数...] => 表达式/块`：

```text
let foo = [x: int] => x + 1;

let bar = [x: int] =>
{
    return x + 1;  // 不可使用 => x + 1，因为闭包涉及到控制流，return 退出的也是闭包内部
}
```

### 代码块省略

单表达式代码块可以使用 `=>` 省略大括号：

```text
// 闭包代码块省略
let foo = [x: int] => x + 1;

// 函数代码块省略（可用于类方法）
func foo(x: int) -> int => x + 1;

// if else 代码块省略
let foo = if x > 0 => 1
          else if x = 0 => 0
          else => -1;

// match 块省略
let foo =
match x
{
case 0 => 0;
case _ => -1;
};

// on 块省略
x on None => recover 0;
```

### 控制流中的值传递

控制流语句使用 `=>` 返回值，禁止隐式返回：

```text
let foo =
if x > 0
{
    => 1;
}
else
{
    return -1;  // 退出当前函数，而非返回 if 块
}
```

### 类与对象

- `class` **仅用于数据封装与静态代码复用**
- **必须能够静态分发**，无 vtable
- **仅支持单继承**
- **禁止隐式类型提升或类型下降**。如下文中 `Bar` 与 `Foo` 是独立的两个类，仅进行了静态代码复用
- 多态应通过 `trait` 实现。运行时多态需主动装箱
- 成员默认 `private`
- 构造函数首参必须为 `self: Self`，返回类型必须为 `Self`，可省略类型首参与返回值的类型标注与 `return` 语句
- 方法若使用 `self`，必须显式声明且为首参，可省略 `Self` 类型，但需注意可变性和引用与转移的标识，不允许 `self` 作为普通参数，也不允许声明为其他类型

```text
class Bar
{
    _z: int;

    Bar(self, z: int)
    {
        self._z = z;
    };
};

class Foo extends Bar
{
    _x: int;  // 默认 private，建议用下划线

public:
    y: int;

    Foo(self, x: int, y: int, z: int)
    {
        super(self, z);
        self._x = x;
        self.y = y;
    };

    // 如果传入的不是引用，lint 会提示
    private getY(&self) -> int  // 临时 private 标记
    {
        // do something...
        return self.y;
    };

    x(&self) -> int => self._x;  // 简单、无副作用的 getter 建议写成类似成员名称的形式，而不用 get 动词。

    consume(self: Self) -> Self
    {
        return self; // 转移所有权
    };
};

trait Printable
{
    print(&self) -> void;
};

impl Printable for Foo
{
    print(&self) -> void
    {
        println("x: {}, y: {}", self.x, self.y);
    };
};
```

### 泛型标记

```text
func foo<T>(x: T) -> T {};
func bar<T impl Printable>(x: T) -> void {};
func baz<T = int>(x: T) -> T {};
func qux<'a, T>(x: &'a T) -> void
where
    T impl Send,
    T outlives 'a
{};
```

### 异步与并发

- `async` 函数返回 `Future<T>`，`await` 解包
- `spawn` 提交并发任务给运行时，返回 `JoinHandle<T>`（`T impl Send`）
- 支持 `await (t1, t2, ...)` 同时等待多个任务，仅在 `await` 中可以使用这个语法糖

#### 执行上下文规则

| 上下文          | `await` 行为          | `spawn` 行为               |
| --------------- | --------------------- | -------------------------- |
| `async` 函数    | 逻辑并发（非阻塞）    | 提交任务给运行时，允许并行 |
| 非 `async` 顶层 | 阻塞（`block_on` 糖） | 阻塞等待，必须 `await`     |
| 普通函数        | 禁止                  | 可返回 `JoinHandle<T>`     |

### 脚本模式与编译模式

- 文件开头注释中加入 `@mode: script` 表示脚本模式
- 脚本模式：
  - 允许顶层副作用、`async/await`
  - 仅用于入口文件时可被 `import`
- 编译模式：
  - 必须有 `main` 函数
  - 不允许顶层副作用
  - 默认行为
- 直接运行解释器默认脚本模式；直接运行文件默认编译模式（除非标记或参数指定）。编译文件一定是编译模式（如果强制指定脚本模式则报错）。

### 错误处理

> // TODO：用户自定义错误处理策略及其他内容工程化支持

在 Lume 中，`None` 和 `Error` 被视为程序执行流中的状态标记，而非可操作的值。这意味着：

- 你不能将 `None` 或 `Error(...)` 赋值给变量
- 你不能将 `None` 或 `Error(...)` 作为函数参数或返回类型的一部分传递
- `Result<T, E>` 和 `Optional<T>` 是状态容器类型，仅用于类型签名，其内部状态只能通过 on、match、return 等专用语法访问

#### 自定义错误种类

提供 `@error` 过程宏用于自定义错误类型。语法使用 `class` 的继承语法，但**语义上是不可实例化的状态标签**。

- 必须作用于继承于另一个 `@error` 标记的类
- 只能含有公开不可变成员（由宏自动实现，不需要主动写 `public` 修饰符，但禁止写 `mut`，`private` 等修饰符）
- 禁止定义任何成员函数，**包括构造函数**
- 成员必须能够拥有 `'static` 生命周期，必须拥有所有权，且可移动（实现 `Move` trait）。

宏行为：

- 自动生成同名不可变结构体，如错误 `MyError` 生成 `MyErrorPayload`
- 自动分析错误继承关系，捕获父错误时可以捕获子错误，但注意子错误信息会被隐藏
- 允许在 return 语句中使用结构字面量语法抛出

```text
/*
根 Error 无任何字段
@error
class Error {}
*/

import { error, Error } from "std/error";

@error
class MyError extends Error
{
    myMessage: String;
    myCode: int;
}

... return MyError { myMessage = "My Error", myCode = 1 };
```

#### 声明与抛出

- 可能抛错的函数需用 `throws` 声明错误类型
  - 可以退回到 `Result<T, E>`，但不推荐
- 抛出错误使用结构式语法：

```text
func foo() -> int throws ValueError, IOError
{
    if ... return ValueError { message = "Value Error"};
    if ... return IOError { message = "IO Error" };
    if ... return Error {};  // 无参数错误可以省略大括号，但建议写全
    return 0;
}
```

#### 处理方式

建议使用 `on` 语法糖来处理错误。

`on` 是 `match` 在错误或空值处理时的**专用**语法糖，本身是一个表达式，其返回类型为 `T`（来自 `Result<T, E>` 或 `Optional<T>`），因此可用于赋值，返回，链式处理等。

##### 方法一：紧跟处理

```text
let x = foo()
on ValueError as e if e.message == "Not Found"  // 可以添加条件
{
    println("Not Found")
    recover 0;  // 恢复为 0，类型需匹配
}
on ValueError  // 必须捕获剩下的可能性
{
    println("Value Error")
    // 如果未进行 recover，错误状态会被保留在 x 里，下次使用时必须处理
}
on IOError  // 不需要可省略捕获
{
    return -1;  // 返回当前函数，也可 return Error 再次抛错，但需要在本函数内声明
};
```

##### 方法二：推迟处理

```text
let r = foo();
let r = r on ValueError => recover 0;  // 使用重绑定
```

- `recover` 表达式需与成功路径一致
- `on` 块可修改外部变量，内部变量结束后销毁

#### 快速传播

`?` 操作符表示快速传播，遇到错误会立刻短路并抛出错误。

```text
let x = a.foo()?.bar()?;
```

#### 链式恢复与 `self`

- 在 `obj.method()` 的 `on` 块中可用 `self`（即 `&obj`）
- 仅在 `obj` 所有权没有被转移时可用（即 `method` 传入的 self 非所有权，也即 `obj.method()` 没有自消费），实际为 &a 的语法糖

  ```text
  a.foo() on IOError
  {
      recover self;  // 注意，这里本身需要 a.foo() 返回类型为 Self（即类型需匹配）
  }.bar() on ValueError
  {
      recover self.x;
  };
  ```

#### 传统 match 模式

所有 `on` 均可等价重写为 `match`。`match` 语法更通用，可处理 `Enum` 等类型，而 `on` 不能。

```text
let x =
match foo()
{
case ValueError as e
{
    println("Value Error: {}", e.message)
    recover 0;
}
case IOError as e
{
    println("IO Error: {}", e.message)
}
case OK as v => v;
};

let mut x?;
let x = match x
{
case None => recover 1;
};
```

### `if case` 语法糖

提供类似 `if let` 的语法糖，与 `match case` 保持风格完全一致。但由于会隐藏错误，故 lint 默认对用于 `Result` 和 `Optional` 的处理发警告。

```text
let color = Color::Red;
let x = if color case Color::Red => 1 else => 0;

/*
展开为
let x =
match color
{
    case Color::Red => 1;
    case _ => 0;
}
*/

// 会报警告，除非显式压制，但可编译
if result() case IOError as e if e.code == 404 => recover 0 else => 1;
```

### `is` 关键字语法糖

引入 `is` 关键字用于状态匹配。

- `is` 是一个返回 bool 的操作符，用于检查状态容器（如 `Optional<T>` 或 `Result<T, E>`）是否处于指定状态且其内容匹配给定模板。它**不产生任何中间值**，右侧的 `Some(...)`、`Error {...}` 等仅作为只读匹配模式存在。可等效展开为 `match`。
- 语法：`expr is pattern`，其中 `pattern` 仅允许状态表达式：

  | 模式                     | 描述                                              |
  | ------------------------ | ------------------------------------------------- |
  | `None`                   | 匹配 `Optional<T>` 的空状态                       |
  | `Some`                   | 匹配 `Optional<T>` 的非空状态                     |
  | `Some(expr)`             | 匹配 `Optional<T>` 的非空状态，且内容等于 `expr`  |
  | `OK`                     | 匹配 `Result<T, E>` 的成功状态                    |
  | `OK(expr)`               | 匹配 `Result<T, E>` 的成功状态，且内容等于 `expr` |
  | `Error`                  | 匹配 `Result<T, E>` 的错误状态                    |
  | `Error { field = expr }` | 匹配 `Result<T, E>` 的错误状态，且所有字段值相等  |

- `expr is pattern` 展开为：

  ```text
  match expr
  {
      case pattern as v if v == ... => true;
      case _ => false;
  };
  ```

- 不能用于非状态类型

### 宏

> // TODO 学习宏语法并完善

#### 声明宏

- 使用 macro 关键字定义声明宏，与 Rust 的宏卫生性一致
- 需使用 `!` 展开宏

```text
export macro log(level, str, args...) {...};

import { log } from "std/log";
log!(INFO, "Hello, world!");
```

#### 过程宏

```text
// 仅语法示例
export macro derive(T) {}

import { derive } from "std/derive";

@derive(Json)
class Foo {}

@config(target == "windows")
func funcForWindows() {}
```

### 其他语言特性

- 支持 `and` / `or` / `not` 逻辑运算符，弃用 `&&` / `||` / `!`。

### 规划

#### 标准库（初步）

- `std/math`：数学函数
- `std/io`：输入输出
- `std/thread`：并发原语（待定）
- `std/result`、`std/optional` 等？

#### 保留计划

- `pure` 用于标记函数无副作用
- `effect` 用于标记函数作用影响范围
