import React, { useEffect, useState, } from 'react';
import { Space, Spin, Tabs, Row, Col, Descriptions, Badge, Image, Card, Table, Tag, Button } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import ReactEcharts from 'echarts-for-react';
// import echarts from 'echarts/lib/echarts';
import { track, vehicle_data, history_trips, history_charges } from '../services/tesla';
import moment from 'moment';
import 'echarts/extension/bmap/bmap.js';
import SmallButton from '../components/SmallButton';
import { useParams } from "react-router-dom";
const { TabPane } = Tabs;


const Track = (props: any) => {
	const { vehicle_id, drive_state } = props;
	let a: any[] = [];
	const [lines, setLines] = useState(a);
	const [counter, setCounter] = useState(0);
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
	}, [vehicle_id, counter]);
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
		<div>
			<Button type="link" onClick={() => {
				setCounter(counter + 1);
			}}>+刷新数据</Button>
		</div>
		<ReactEcharts
			option={getOption()}
			style={{ height: '880px', }}
		/>
	</div>
}

const vehicle_picture = (color: String) => {
	if (color.toUpperCase().indexOf("BLACK") > 0) {
		return "https://static-assets.tesla.cn/configurator/compositor?context=design_studio_2&options=$MTY18,$PBSB,$WY19C,$INPB2&view=FRONT34&model=my&size=1920&bkba_opt=2&crop=0,0,0,0&"
	}
	return "https://static-assets.tesla.cn/configurator/compositor?context=design_studio_2&options=$MTY13,$PPMR,$WY19B,$INPB0&view=FRONT34&model=my&size=1920&bkba_opt=2&crop=0,0,0,0&";
}

const Overview = (props: any) => {
	const { vin, state, vehicle_config, vehicle_state, climate_state, drive_state, charge_state } = props;
	let status = state === "online" ? <Badge status="processing" text={state} /> : <Badge status="default" text={state} />;
	return (<div>
		<Card>
			<Row>
				<Col span={8}>
					<Image preview={false} src={vehicle_picture(vehicle_config.exterior_color)}></Image>
				</Col>
				<Col span={16}>
					<Descriptions size='small' column={1} bordered contentStyle={{ textAlign: 'right' }}>
						<Descriptions.Item label="State">{status}</Descriptions.Item>
						<Descriptions.Item label="车架号Vin">{vin}</Descriptions.Item>
						<Descriptions.Item label="Car Type">{vehicle_config.car_type}</Descriptions.Item>
						<Descriptions.Item label="里程表">{(vehicle_state.odometer * 1.6093).toFixed(0)}</Descriptions.Item>
						<Descriptions.Item label="内部温度">{climate_state.inside_temp} ℃</Descriptions.Item>
						<Descriptions.Item label="外部温度">{climate_state.outside_temp} ℃</Descriptions.Item>
						<Descriptions.Item label="更新时间">{moment.unix(vehicle_state.timestamp / 1000).format('YYYY-MM-DD HH:mm:ss')}</Descriptions.Item>
						<Descriptions.Item label="剩余电量">{charge_state.battery_level}%</Descriptions.Item>
					</Descriptions>
				</Col>
			</Row>
		</Card>

	</div>)
}

const HistoryCharges = (props: any) => {
	const { vehicle_id, drive_state } = props;
	const [chargeList, setChargeList] = useState([]);
	const [currentIndex, setCurrentIndex] = useState(-1);
	useEffect(() => {
		history_charges(vehicle_id).then(res => {
			console.log("history charges ", res);
			setChargeList(res.history_charges);
		})
		return () => { };
	}, [vehicle_id]);

	if (vehicle_id === 0) {
		return (<div></div>);
	}
	interface DataType {
		startTime: number;
		endTime: number;
		key: number,
		chargeEnergyAdded: number
		rangeAdded: number
	}

	const columns: ColumnsType<DataType> = [
		{
			title: '起始',
			dataIndex: 'startTime',
			key: 'start-time',
			render: (t) => <a>{
				moment.unix(t / 1000).format('YYYY-MM-DD HH:mm:ss')
			}</a>,
		},
		{
			title: '结束',
			dataIndex: 'endTime',
			key: 'end-time',
			render: (t) => <a>{
				moment.unix(t / 1000).format('YYYY-MM-DD HH:mm:ss')
			}</a>,
		},
		{
			title: '电量增加(kw)',
			dataIndex: 'chargeEnergyAdded',
			key: 'charge_energy_added',
		},
		{
			title: '里程增加(km)',
			key: 'range_added',
			render: (row: any) => {
				return <a>{row.rangeAdded.toFixed(1)}</a>;
			}
		},
		{
			title: '查看',
			render: (row: any) => {
				return <SmallButton type="link" onClick={() => {
					setCurrentIndex(row.key);
				}}>查看明细</SmallButton>
			}
		}
	];
	let get_data = () => {
		return chargeList.map((c: any, index) => {
			let first = c.details[0];
			let last = c.details[c.details.length - 1];
			let d: DataType = {
				startTime: first.timestamp,
				endTime: last.timestamp,
				key: index,
				chargeEnergyAdded: last.charge_energy_added,
				rangeAdded: last.charge_miles_added_ideal * 1.609
			}
			return d;
		});
	}

	return <div>
		<Row>
			<Col span={12}>
				<Table columns={columns} dataSource={get_data()} />
			</Col>
			<Col span={12}>
			</Col>
		</Row>
	</div>
}

const HistoryTrips = (props: any) => {
	const { vehicle_id, drive_state } = props;
	const [trips, setTrips] = useState([]);
	const [currentTrackIndex, setCurrentTrackIndex] = useState(-1);
	let a: any[] = [];
	useEffect(() => {
		history_trips(vehicle_id).then(res => {
			console.log("history charges ", res);
			setTrips(res.trips);
		})
		return () => { };
	}, [vehicle_id]);
	console.log(trips, currentTrackIndex);

	if (vehicle_id === 0) {
		return (<div></div>);
	}
	interface DataType {
		startTime: number;
		endTime: number;
		key: number,
	}

	const columns: ColumnsType<DataType> = [
		{
			title: '起始',
			dataIndex: 'startTime',
			key: 'start-time',
			render: (t) => <a>{
				moment.unix(t / 1000).format('YYYY-MM-DD HH:mm:ss')
			}</a>,
		},
		{
			title: '结束',
			dataIndex: 'endTime',
			key: 'end-time',
			render: (t) => <a>{
				moment.unix(t / 1000).format('YYYY-MM-DD HH:mm:ss')
			}</a>,
		},
		{
			title: '查看',
			render: (row: any) => {
				return <SmallButton type="link" onClick={() => {
					setCurrentTrackIndex(row.key);
				}}>查看轨迹</SmallButton>
			}
		}
	];
	let get_data = () => {
		return trips.map((trip: any, index) => {
			let last = trip.track[trip.track.length - 1];
			let d: DataType = {
				startTime: trip.track[0].timestamp,
				endTime: last.timestamp,
				key: index,
			}
			return d;
		});
	}
	const get_lines = () => {
		let coords: any[] = [];
		if (currentTrackIndex < 0 || currentTrackIndex > trips.length) {
			return coords;
		}
		console.log("trips", trips, currentTrackIndex);
		const track = (trips[currentTrackIndex] as any).track;
		for (let i = 0; i < track.length; i++) {
			coords.push([track[i].longitude, track[i].latitude]);
		}
		if (coords.length > 0) {
			return [{ coords }];
		}
	}
	const get_center = () => {
		let coords: any[] = [];
		if (currentTrackIndex < 0 || currentTrackIndex > trips.length) {
			return [drive_state.longitude, drive_state.latitude];
		}
		const track = (trips[currentTrackIndex] as any).track;
		return [track[0].longitude, track[0].latitude];
	}
	const getOption = () => {
		const option = {
			bmap: {
				center: get_center(),
				zoom: 14,
				roam: true,
				mapStyle: {

				}
			},
			series: [
				{
					type: 'lines',
					coordinateSystem: 'bmap',
					data: get_lines(),
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
	return <div>
		<Row>
			<Col span={8}>
				<Table columns={columns} dataSource={get_data()} />
			</Col>
			<Col span={16}>
				<ReactEcharts
					option={getOption()}
					style={{ height: '780px', }}
				/>
			</Col>
		</Row>
	</div>
}

export default () => {
	const [loading, setLoading] = useState(false);
	const { vehicle_id } = useParams<{ vehicle_id?: string }>();
	const [vehicleData, setVehicleData] = useState({
		vehicle_id: 0,
		state: "",
		drive_state: { timestamp: 0, longtitude: 121.553662, latitude: 31.194845 },
		vehicle_state: { car_version: "", odometer: 0.0, vehicle_name: "", timestamp: 0 },
		vehicle_config: { car_type: "", exterior_color: "SolidBlack" },
		climate_state: { outside_temp: "", inside_temp: "" },
		gui_settings: {},
		charge_state: { battery_level: 0.0 }
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
		<Card title={vehicleData.vehicle_state.vehicle_name}>
			<Tabs style={{ width: "100%" }} defaultActiveKey="overview" onChange={() => { }}>
				<TabPane tab="概览" key="overview" >
					<Overview {...vehicleData}></Overview>
				</TabPane>
				<TabPane tab="管理" key="management" ></TabPane>
				<TabPane tab="充电" key="charge" >
					<HistoryCharges {...vehicleData}></HistoryCharges>
				</TabPane>
				<TabPane tab="旅程" key="trip" >
					<HistoryTrips {...vehicleData}></HistoryTrips>
				</TabPane>
				<TabPane tab="足迹" key="track" >
					<Track {...vehicleData} ></Track>
				</TabPane>
			</Tabs>
		</Card>

	</Spin>
}