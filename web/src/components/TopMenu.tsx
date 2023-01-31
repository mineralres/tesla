import React, { useState } from 'react'
import { Menu } from 'antd';
import { SettingFilled } from '@ant-design/icons';


const { SubMenu } = Menu;

export default () => {
    const [key, setKey] = useState('mail');
    let handleClick = (e: any) => {
        console.log('click ', e);
        setKey(e.key)
    };

    return (
        <Menu onClick={handleClick} selectedKeys={[key]} mode="horizontal">
            <SubMenu title="其他">
                <Menu.ItemGroup title="行情">
                    <Menu.Item key="setting:4x33">
                        <a href="/about">
                            关于
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
    );
}
