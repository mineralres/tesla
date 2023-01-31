import { post } from './ajax'

export const track = () => {
	return post('/api/tesla/track', {})
}