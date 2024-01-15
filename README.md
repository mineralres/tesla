# tesla
这是一个和teslamate类似的 tesla车子数据缓存和统计的工具，后端使用rust实现，前端用的是react框架.

* 直接保存数据，不需要运行三方数据库，方便部署
* 带一个可视化web端
* 不会主动唤醒车辆
* 后端用rust实现, web使用react框架

### build
```
cargo build --release
cd web
npm run build 
```

### run
```
./target/release/app configs/app.json
```

### 说明
1. 获取token, 推荐使用 https://github.com/adriankumpf/tesla_auth
2. 支持记录tesla账户下的全部车辆数据
3. 记录的数据包括drive_state, climate_state, charge_state,和steam推送的实时数据(车子处于活跃状态时会推送，包括gps坐标，海拔，soc，power等)
4. 不会主动唤醒车辆