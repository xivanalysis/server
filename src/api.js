import Koa from 'koa'
import Router from 'koa-router'

import redis from './redis'

const api = new Koa()
const router = new Router()

router.get('/', async ctx => {
	ctx.body = await redis.get('test')
})

api.use(router.routes())
api.use(router.allowedMethods())

export default api
