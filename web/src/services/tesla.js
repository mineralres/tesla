import { post } from './ajax'

export const track = (id) => {
	return post('/api/tesla/track', { id })
}

export const vehicles = () => {
	return post('/api/tesla/vehicles', {})
}

export const vehicle_data = (id) => {
	return post('/api/tesla/vehicle_data', { id })
}

export const user_me = () => {
	return post('/api/tesla/user_me', {})
}

