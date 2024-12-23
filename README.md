# miio-light-switch

使用 esp-rs 和米家集成的智能开关固件。

## 开始

### 前置条件

- [Rust](https://www.rust-lang.org/tools/install)
- [esp-rs](https://docs.esp-rs.org/book/) 工具链

### 米家

- 创建小米米家产品，产品配网方式为 Combo 配网模式，功能定义见 [spec.json](./spec.json)
- 根据产品信息配置 `main.rs` 中的代码
    ```rust
    let mut miio = crate::miio::IoTFramework::new(
        peripherals.uart1, 
        pins.gpio12, 
        pins.gpio11,
        "csbupt.switch.smsw", // model
        "0001", // version
        "24351" // pid
    )?;
    ```
- （可选）在米家开发者平台—高阶配置-消息推送/自动化配置中配置高级功能

### 硬件

- gpio1: 连接到光线传感器的模拟输出
- gpio9: 连接到舵机的 pwm 输入
- gpio12: 串口 tx - 与米家模块的 rx 连接
- gpio11: 串口 rx - 与米家模块的 tx 连接

### 编译

首先编译前端（连接校园网部分）

```bash
pushd frontend
pnpm i
pnpm run build
popd
```

然后编译固件

```bash
cargo run
```

### cargo features

- restore: 上电后重置米家模块到出厂状态
- clean_nvs：清除 nvs 存储，删掉保存在 flash 中的校园网账号和密码
- random_mac：随机生成 mac 地址，相当于登出校园网

### 配置

#### 登录校园网

上电后，通过手机连接到 `smart-light` WIFI，手机自动打开 `http://192.168.71.1`，输入校园网账号和密码，点击登录。

#### 绑定到米家

手机打开米家 APP，添加设备，选择米家开发者平台创建的产品，按照提示操作。