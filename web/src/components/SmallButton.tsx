import React, { useState } from 'react';
import { Button } from 'antd';

export default (props: any) => {
	const { children, onClick, duration, type } = props;
	const [loading, setLoading] = useState(false);
	return <Button type={type} loading={loading} onClick={(e) => {
		setLoading(true)
		if (duration) {
			setTimeout(() => { setLoading(false) }, duration)
		} else {
			setTimeout(() => { setLoading(false) }, 500)
		}
		if (onClick) {
			onClick(e)
		}
	}}>{children}</Button>
}
