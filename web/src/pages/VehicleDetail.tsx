import React, { useEffect, useState, } from 'react';
import { Spin, Tabs, Row, Col, Descriptions, Badge, Image, Card } from 'antd';
import ReactEcharts from 'echarts-for-react';
// import echarts from 'echarts/lib/echarts';
import { track, vehicle_data } from '../services/tesla';
import moment from 'moment';
import 'echarts/extension/bmap/bmap.js';
import { useParams } from "react-router-dom";
const { TabPane } = Tabs;


const Track = (props: any) => {
	const { vehicle_id, drive_state } = props;
	let a: any[] = [];
	const [lines, setLines] = useState(a);
	useEffect(() => {
		track(vehicle_id).then(res => {
			let coords: any[] = [];
			for (let i = 0; i < res.longitude.length; i++) {
				coords.push([res.longitude[i], res.latitude[i]]);
			}
			if (coords.length > 0) {
				setLines([{ coords }]);
			}
		});
		return () => { };
	}, [vehicle_id]);
	const getOption = () => {
		const option = {
			bmap: {
				center: [drive_state.longitude, drive_state.latitude],
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
						color: 'green',
						opacity: 0.9,
						width: 1
					}
				}
			]
		};
		return option;
	}

	if (vehicle_id === 0) {
		return (<div></div>);
	}
	return <div>
		<ReactEcharts
			option={getOption()}
			style={{ height: '880px', }}
		/>
	</div>
}

const Overview = (props: any) => {
	const { vin, state, vehicle_config, vehicle_state, climate_state, drive_state } = props;
	let status = state === "online" ? <Badge status="processing" text={state} /> : <Badge status="default" text={state} />;
	return (<div>
		<Card>
			<Row>
				<Col span={8}>
					<Image preview={false} src="https://static-assets.tesla.cn/configurator/compositor?context=design_studio_2&options=$MTY13,$PPMR,$WY19B,$INPB0&view=FRONT34&model=my&size=1920&bkba_opt=2&crop=0,0,0,0&"></Image>
				</Col>
				<Col span={5}>
					<Descriptions size='small' column={1} bordered contentStyle={{ textAlign: 'right' }}>
						<Descriptions.Item label="State">{status}</Descriptions.Item>
						<Descriptions.Item label="Vin">{vin}</Descriptions.Item>
						<Descriptions.Item label="Car Type">{vehicle_config.car_type}</Descriptions.Item>
						<Descriptions.Item label="Odometer">{(vehicle_state.odometer * 1.6093).toFixed(0)}</Descriptions.Item>
						<Descriptions.Item label="Inside Temperature">{climate_state.inside_temp} ℃</Descriptions.Item>
						<Descriptions.Item label="Outside Temperature">{climate_state.outside_temp} ℃</Descriptions.Item>
						<Descriptions.Item label="Update Time">{moment.unix(drive_state.timestamp / 1000).format('YYYY-MM-DD HH:mm:ss')}</Descriptions.Item>
					</Descriptions>
				</Col>
			</Row>
		</Card>

	</div>)
}

export default () => {
	const [loading, setLoading] = useState(false);
	const { vehicle_id } = useParams<{ vehicle_id?: string }>();
	const [vehicleData, setVehicleData] = useState({
		vehicle_id: 0,
		state: "",
		drive_state: { timestamp: 0, longtitude: 121.553662, latitude: 31.194845 },
		vehicle_state: { car_version: "", odometer: 0.0 },
		vehicle_config: { car_type: "" },
		climate_state: { outside_temp: "", inside_temp: "" }
	});

	useEffect(() => {
		setLoading(true);
		vehicle_data(Number(vehicle_id)).then(res => {
			setLoading(false);
			setVehicleData(res);
		});
		return () => { };
	}, [vehicle_id]);
	return <Spin spinning={loading}>
		<Tabs style={{ width: "100%" }} defaultActiveKey="overview" onChange={() => { }}>
			<TabPane tab="概览" key="overview" >
				<Overview {...vehicleData}></Overview>
			</TabPane>
			<TabPane tab="管理" key="management" ></TabPane>
			<TabPane tab="充电" key="charge" ></TabPane>
			<TabPane tab="旅程" key="trip" ></TabPane>
			<TabPane tab="足迹" key="track" >
				<Track {...vehicleData} ></Track>
			</TabPane>
		</Tabs>
	</Spin>
}