import React, { useEffect, useState, } from 'react';
import { Form, Button, Input } from 'antd';
import { Link } from "react-router-dom";
import { set_api_token, vehicles } from '../services/tesla';
import 'echarts/extension/bmap/bmap.js';

export default () => {
	const [form] = Form.useForm();
	useEffect(() => {
		return () => {
		}
	}, []);
	const onFinish = (values: any) => {
		console.log("onFinish", values);
		set_api_token(values.access_token, values.refresh_token).then((res) => {
			console.log("res", res);
		});
	};

	const onReset = () => {
		form.setFieldsValue({ access_token: "", refresh_token: "" });
	};
	const layout = {
		labelCol: { span: 8 },
		wrapperCol: { span: 16 },
	};

	const tailLayout = {
		wrapperCol: { offset: 8, span: 16 },
	};

	return (
		<div>
			<Form
				{...layout}
				form={form}
				name="control-hooks"
				onFinish={onFinish}
				style={{ maxWidth: 600 }}
			>
				<Form.Item name="access_token" label="access_token" rules={[{ required: true }]}>
					<Input />
				</Form.Item>
				<Form.Item name="refresh_token" label="refresh_token" rules={[{ required: true }]}>
					<Input />
				</Form.Item>
				<Form.Item {...tailLayout}>
					<Button type="primary" htmlType="submit">
						提交
					</Button>
					<Button htmlType="button" onClick={onReset}>
						重置
					</Button>
				</Form.Item>
			</Form>

		</div>
	)
	return <>SetApiToken</>
}