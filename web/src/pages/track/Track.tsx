import React, { useEffect, useState } from 'react';
import { Row, Col } from 'antd';
import ReactEcharts from 'echarts-for-react';
import { track, vehicles, vehicle_data } from '../../services/tesla';

// 请确保在引入百度地图扩展之前已经引入百度地图 JS API 脚本并成功加载
// https://api.map.baidu.com/api?v=3.0&ak=你申请的AK
import 'echarts/extension/bmap/bmap.js';

export default () => {
    let a: any[] = [];
    const [lines, setLines] = useState(a);
    useEffect(() => {
        vehicles().then(res => {
            console.log("vehicles", res);
            if (res.length > 0) {
                vehicle_data(res[0].vehicle_id).then(res => {
                    console.log("vehicle_data", res);
                });
            }
        });
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