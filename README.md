# Lume 语言设计文档（v0.2）

## 综述

Lume 是一个**强类型、内存安全、多范式**的编程语言，目标是融合现代语言的易用性与系统语言的性能和安全性。~~实际上是 Rust++~~

### 核心特性

- **语法风格**：类 Dart / TypeScript，使用四空格缩进（非强制）
- **模块系统**：ECMAScript Module（`import` / `export`）
- **内存模型**：类似 Rust 的所有权与生命周期机制，**无垃圾回收（GC-free）**
- **类型哲学**：
  - Everything is Object（所有值皆为对象），但对基本类型做底层优化
  - `class` 仅用于继承与数据封装，**多态由 `trait` 实现**
  - `Error` `None` 为**状态**而非**值**
- **执行方式**：支持编译（AOT）与解释（JIT/REPL）两种模式

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

> own 关键字作用于变量，表示 “交出变量的所有权”

```text
let a = 1;  // ✅正确，1 （i32）可 Copy
let b = "hello";  // ✅正确，字面量 StringSlice 被 Copy
let c = a;  // ✅正确，此时 a、c 都有效

let d = "hello".toString();  // 生成 String
let e = d;  // ❌错误，无法复制 String，需要显式转移所有权
let e = own b;  // ✅正确，显示转移 b 的所有权，b 失效

func foo(own a: Bar) -> Bar  // 显式要求 a 具有所有权，返回值不需要写 own
{
    return a;  // 返回值自动转移
};

let f = Bar();
let g = foo(own f);  // 参数与调用均需显式标注所有权传递
```

### 生命周期

- 使用 `'a`、`'b` 等标记生命周期
- 生命周期标注在**类型**上，如 `'a &T`
- 使用 `outlives` 约束生命周期长短关系

```text
func maxRef<'short, 'long>(x: 'short &Foo, y: 'long &Foo) -> 'long &Foo
where
    'long outlives 'short
{
    if x.a > y.a => return x;
    else => return y;
}

class 'a Bar
{
    value: 'a &int;
}
```

### 模块系统

- 与 ECMAScript Modules 几乎一致
- 引入 `link`： `static` / `dynamic` 控制链接方式：

  ```text
  import { foo } from "./mod.lume" with { link: "static" };  // 静态链接（默认，可省略）
  import { bar } from "./mod.lume" with { link: "dynamic" };  // 动态链接（需预编译为动态库）
  export func baz() { ... }
  ```

- 特殊导入语法：
  - impl：`import { impl Foo for Bar } from "./mod.lume";`

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
- 构造函数首参必须为 `own self: Self`，返回类型必须为 `Self`，可省略类型首参与返回值的类型标注与 `return` 语句
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

    consume(own self: Self) -> Self
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
func qux<T>(x: T) -> void
where
    T impl Send,
    T outlives 'static
{};
```

### 异步与并发

- `async` 函数返回 `Future<T>`，`await` 解包
- `spawn` 提交并发任务给运行时，返回 `JoinHandle<T>`（`T: Send`）
- 支持 `await (t1, t2, ...)` 同时等待多个任务，仅在 `await` 中可以使用这个语法糖

#### 执行上下文规则

| 上下文          | `await` 行为          | `spawn` 行为               |
| --------------- | --------------------- | -------------------------- |
| `async` 函数    | 逻辑并发（非阻塞）    | 提交任务给运行时，允许并行 |
| 非 `async` 顶层 | 阻塞（`block_on` 糖） | 阻塞等待，必须 `await`     |
| 普通函数        | ❌ 禁止               | 可返回 `JoinHandle<T>`     |

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

... return MyError { myMessage: "My Error", myCode: 1 };
```

#### 声明与抛出

- 可能抛错的函数需用 `throws` 声明错误类型
  - 可以退回到 `Result<T, E>`，但不推荐
- 抛出错误使用结构式语法：

```text
func foo() -> int throws ValueError, IOError
{
    if ... return ValueError { message: "Value Error"};
    if ... return IOError { message: "IO Error" };
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
let x = if color case Color::Red => 1 else 0;

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
if result() case IOError as e if e.code == 404 => recover 0 else 1;
```

### `is` 关键字语法糖

引入 `is` 关键字用于状态匹配。

- `is` 是一个返回 bool 的操作符，用于检查状态容器（如 `Optional<T>` 或 `Result<T, E>`）是否处于指定状态且其内容匹配给定模板。它**不产生任何中间值**，右侧的 `Some(...)`、`Error {...}` 等仅作为只读匹配模式存在。可等效展开为 `match`。
- 语法：`expr is pattern`，其中 `pattern` 仅允许状态表达式：

  | 模式                    | 描述                                              |
  | ----------------------- | ------------------------------------------------- |
  | `None`                  | 匹配 `Optional<T>` 的空状态                       |
  | `Some`                  | 匹配 `Optional<T>` 的非空状态                     |
  | `Some(expr)`            | 匹配 `Optional<T>` 的非空状态，且内容等于 `expr`  |
  | `OK`                    | 匹配 `Result<T, E>` 的成功状态                    |
  | `OK(expr)`              | 匹配 `Result<T, E>` 的成功状态，且内容等于 `expr` |
  | `Error`                 | 匹配 `Result<T, E>` 的错误状态                    |
  | `Error { field: expr }` | 匹配 `Result<T, E>` 的错误状态，且所有字段值相等  |

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
