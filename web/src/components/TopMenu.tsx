import React, { useState, useEffect } from 'react'
import { Menu, Avatar, Row, Col, Space, Image } from 'antd';
import { SettingFilled } from '@ant-design/icons';
import { user_me } from "../services/tesla";


const { SubMenu } = Menu;

export default () => {
    const [key, setKey] = useState('mail');
    const [userMe, setUserMe] = useState({ profile_image_url: "" });
    let handleClick = (e: any) => {
        console.log('click ', e);
        setKey(e.key)
    };
    useEffect(() => {
        user_me().then(res => {
            console.log(res);
            setUserMe(res);
        });
        return () => { };
    }, []);

    return (
        <Row>
            <Col>
                <Menu onClick={handleClick} selectedKeys={[key]} mode="horizontal">
                    <SubMenu title="其他">
                        <Menu.ItemGroup title="行情">
                            <Menu.Item key="setting:4x33">
                                <a href="/about">
                                    关于
                                </a>
                            </Menu.Item>
                            <Menu.Item key="vehicles:ii">
                                <a href="/vehicles">
                                    车
                                </a>
                            </Menu.Item>
                            <Menu.Item key="setting:4x33:tract">
                                <a href="/track">
                                    路径
                                </a>
                            </Menu.Item>
                        </Menu.ItemGroup>
                    </SubMenu>
                </Menu>
            </Col>
            <Col>
                <Space direction="vertical" wrap size={16}>
                    <Avatar src={userMe.profile_image_url} ></Avatar>
                </Space>
            </Col>
        </Row>

    );
}
