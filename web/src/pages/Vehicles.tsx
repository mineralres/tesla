import React, { useEffect, useState, } from 'react';
import { Row, Col, Badge, Image, Space, Card, Spin, Descriptions } from 'antd';
import { Link } from "react-router-dom";
import { vehicles } from '../services/tesla';
import 'echarts/extension/bmap/bmap.js';

const VehicleCard = (props: any) => {
	console.log('props', props);
	const { vehicle_id, display_name, state, vin } = props;
	useEffect(() => {
		return () => { };
	}, []);
	let status = state === "online" ? <Badge status="processing" text={state} /> : <Badge status="default" text={state} />;
	return (<div>
		<Link to={`/vehicledetail/${vehicle_id}`}>
			<Card title={display_name} hoverable
				style={{ width: 600 }}  >
				<Row>
					<Col span={8}>
						<Image preview={false} src="https://static-assets.tesla.cn/configurator/compositor?context=design_studio_2&options=$MTY13,$PPMR,$WY19B,$INPB0&view=FRONT34&model=my&size=1920&bkba_opt=2&crop=0,0,0,0&"></Image>
					</Col>
					<Col span={16}>
						<Descriptions size='small' column={1} bordered contentStyle={{ textAlign: 'right' }}>
							<Descriptions.Item label="State">{status}</Descriptions.Item>
							<Descriptions.Item label="Vin">{vin}</Descriptions.Item>
							{/* <Descriptions.Item label="Car Type">{vehicleData.vehicle_config.car_type}</Descriptions.Item>
						<Descriptions.Item label="Odometer">{(vehicleData.vehicle_state.odometer * 1.6093).toFixed(0)}</Descriptions.Item>
						<Descriptions.Item label="Inside Temperature">{vehicleData.climate_state.inside_temp} ℃</Descriptions.Item>
						<Descriptions.Item label="Outside Temperature">{vehicleData.climate_state.outside_temp} ℃</Descriptions.Item>
						<Descriptions.Item label="Update Time">{moment.unix(vehicleData.drive_state.timestamp / 1000).format('YYYY-MM-DD HH:mm:ss')}</Descriptions.Item> */}
						</Descriptions>
					</Col>
				</Row>
			</Card>
		</Link></div>)
}

export default () => {
	const [loading, setLoading] = useState(false);
	const [vehicleList, setVehicleList] = useState([]);
	useEffect(() => {
		setLoading(true);
		vehicles().then(res => {
			setLoading(false);
			setVehicleList(res);
		});
		return () => { };
	}, []);
	return <Spin spinning={loading}>
		<Space direction="vertical" size={16}>
			{
				vehicleList.map((e: any, index) =>
					<VehicleCard key={index} {...e}>
					</VehicleCard>
				)
			}
		</Space>
	</Spin>
}