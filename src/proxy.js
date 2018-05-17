import axios from 'axios'
import Koa from 'koa'

import redis from './redis'

const proxy = new Koa()

// FFLogs proxy
const fflogsApi = axios.create({
	baseURL: 'https://www.fflogs.com/v1/'
})

proxy.use(async ctx => {
	// Default to our api key, but allow overrides
	let apiKey = process.env.FFLOGS_API_KEY
	const query = ctx.query
	if (query.api_key) {
		apiKey = query.api_key
		delete query.api_key
		ctx.query = query
	}

	// Build the URL that we'll be requesting (and using as a key!)
	const url = ctx.request.url

	// Check if we have a cached copy, return that if we do
	const key = 'proxy:fflogs:' + url
	if (await redis.exists(key)) {
		ctx.body = await redis.get(key)
		return
	}

	// No cached copy, grab it down
	let response = null
	try {
		response = await fflogsApi.get(url, {
			params: { api_key: apiKey }
		})
	} catch (e) {
		// TODO: Handle this better.
		ctx.body = JSON.stringify([e.response.status, e.response.data])
		return
	}

	// Save and respond
	const data = response.data
	redis.set(key, JSON.stringify(data))
	ctx.body = data
})

export default proxy
