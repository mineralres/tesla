import React, { useState, useEffect } from 'react';
import { Image, Layout, Menu, Avatar, Divider, Row, Col } from 'antd';
import './App.css';
import { BrowserRouter as Router, Switch, Route, } from "react-router-dom";
import Track from './pages/track/Track';
import Vehicles from './pages/Vehicles';
import VehicleDetail from './pages/VehicleDetail';
import SetApiToken from './pages/SetApiToken';
import { user_me } from "./services/tesla";

const { Content, Footer, Sider } = Layout;

const About = () => {
  return (
    <div>
      <h1>Drive the Future!</h1>
      <li>
        <a style={{}} href="https://github.com/mineralres">代码</a>
      </li>
      <Row>
        <Image style={{ width: '50%' }} preview={false} src="https://tesla-cdn.thron.com/delivery/public/image/tesla/20bf4269-45ae-4473-8133-00c144d7265b/bvlatuR/std/2880x1800/Homepage-Model-3-Hero-Desktop-CN?quality=auto-medium&format=auto"></Image>
      </Row>
    </div>
  )
}

export default () => {
  const [collapsed, setCollapsed] = useState(true);
  const [userMe, setUserMe] = useState({ profile_image_url: "" });
  const [key, setKey] = useState('mail');
  useEffect(() => {
    user_me().then(res => {
      console.log('user me ', res);
      setUserMe(res);
    });
    return () => { };
  }, []);

  return (
    <Router>
      <Layout style={{ minHeight: '100vh' }}>
        <Sider theme="light" collapsible collapsed={collapsed} onCollapse={(value) => setCollapsed(value)}>
          <Avatar src={userMe.profile_image_url} style={{ margin: "10px" }} ></Avatar>
          <Divider style={{ margin: "0px" }}></Divider>
          <Menu onClick={(e: any) => {
            setKey(e.key)
            console.log("e.key=", e.key);
          }} selectedKeys={[key]}
            theme="light"
            mode="inline"
          >
            <Menu.Item key="vehicles:ii">
              <a href="/vehicles">
                车辆
              </a>
            </Menu.Item>
            <Menu.Item key="setting:4x33">
              <a href="/about">
                关于
              </a>
            </Menu.Item>
            <Menu.Item key="setting:554x33">
              <a href="/set_api_token">
                登陆Tesla账户
              </a>
            </Menu.Item>
          </Menu>
        </Sider>
        <Layout className="site-layout">
          <Content style={{ margin: '0 16px' }}>
            <Switch>
              <Route path="/about">
                <About></About>
              </Route>
              <Route path="/track">
                <Track></Track>
              </Route>
              <Route path="/vehicles">
                <Vehicles></Vehicles>
              </Route>
              <Route path="/vehicledetail/:vehicle_id">
                <VehicleDetail></VehicleDetail>
              </Route>
            </Switch>
            <Route path="/set_api_token">
              <SetApiToken></SetApiToken>
            </Route>
          </Content>
          <Footer style={{ textAlign: 'center' }}>
            <a href="https://github.com/mineralres">Tesla api tools ©2023 Created by String</a>
          </Footer>
        </Layout>
      </Layout>
    </Router>
  );
};
