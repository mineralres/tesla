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

export const history_charges = (id) => {
	return post('/api/tesla/history_charges', { id })
}


export const history_trips = (id) => {
	return post('/api/tesla/history_trips', { id })
}

export const user_me = () => {
	return post('/api/tesla/user_me', {})
}

