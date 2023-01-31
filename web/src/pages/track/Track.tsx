import React, { useEffect, useState, useContext, useReducer, useRef } from 'react';
import { Row, Col, Divider, Select, Card, InputNumber, Table, Input, Spin, Checkbox, Tabs, Button } from 'antd';
import ReactEcharts from 'echarts-for-react';
import echarts from 'echarts/lib/echarts';

import { track } from '../../services/tesla';

// 请确保在引入百度地图扩展之前已经引入百度地图 JS API 脚本并成功加载
// https://api.map.baidu.com/api?v=3.0&ak=你申请的AK
import 'echarts/extension/bmap/bmap.js';

export default () => {
    let a: any[] = [];
    const [lines, setLines] = useState(a);
    useEffect(() => {
        track().then(res => {
            let coords: any[] = [];
            res.footprint.forEach((e: any) => {
                coords.push([e.longitude, e.latitude])
            });
            setLines([{ coords }]);
        });
        return () => { };
    }, []);

    const getOption = () => {
        console.log("lines", lines);
        const option = {
            bmap: {
                center: [121.553662, 31.194845],
                zoom: 14,
                roam: true,
                mapStyle: {

                }
            },
            series: [
                {
                    type: 'lines',
                    coordinateSystem: 'bmap',
                    data: lines,
                    polyline: true,
                    lineStyle: {
                        color: 'purple',
                        opacity: 0.9,
                        width: 1
                    }
                }
            ]
        };
        return option;
    }

    return <div>
        <Row>
            <Col span={24}>
                <ReactEcharts
                    option={getOption()}
                    style={{ height: '1030px', }}
                />
            </Col>
        </Row>

    </div>
}