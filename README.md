# tesla
这是一个和teslamate类似的 tesla车子数据缓存和统计的工具，后端使用rust实现，前端用的是react框架.

### build
```
cargo build --bin maind --release
cd web
npm run build 
```

### run
create new file in .cache/pass.txt, firstl line is your email, second line is your password
then
```
./target/release/maind
```

### 说明
1. 账号和密码放在.cache/pass.txt里面，第一行是账号，第二行是密码. (如果没有.cache目录请先手动创建)
2. 登陆时会查询账户下的车子列表, 目前只订阅第一个车的推送数据.
