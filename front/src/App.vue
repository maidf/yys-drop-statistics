<template>
    <el-container>
        <el-header>
            <h1>YYS掉落统计</h1>
        </el-header>

        <el-main>
            <!-- 添加资源类型 -->
            <el-card class="box-card">
                <h2>添加掉落类型</h2>
                <el-input v-model="newResourceType" placeholder="资源名称" class="input-with-button" />
                <el-button type="primary" @click="addResourceType">添加资源类型</el-button>
            </el-card>

            <!-- 添加活动 -->
            <el-card class="box-card">
                <h2>添加活动</h2>
                <el-input v-model="newActivity" placeholder="活动名称" class="input-with-button" />
                <el-button type="primary" @click="addActivity">添加活动</el-button>
            </el-card>

            <!-- 批量添加资源记录 -->
            <el-card class="box-card" v-if="resourceTypes.length > 0 && activities.length > 0">
                <h2>添加掉落记录</h2>
                <el-select v-model="selectedActivity" placeholder="选择活动" style="width: 100%; margin-bottom: 10px">
                    <el-option v-for="activity in activities" :key="activity.id" :label="activity.name"
                        :value="activity.id" />
                </el-select>
                <el-input-number v-model="staminaCost" placeholder="每次提交的体力消耗" :min="0"
                    style="width: 100%; margin-bottom: 10px" />
                <el-table :data="batchRecords" style="width: 100%; margin-bottom: 10px">
                    <el-table-column prop="resourceTypeName" label="资源类型" />
                    <el-table-column label="数量">
                        <template #default="scope">
                            <el-input-number v-model="scope.row.amount" :min="0" size="small" />
                        </template>
                    </el-table-column>
                </el-table>
                <el-button type="success" @click="submitBatchRecords">添加</el-button>
            </el-card>

            <!-- 每个活动一个表格 -->
            <div v-for="activity in activities" :key="activity.id" class="activity-section">
                <el-card class="box-card">
                    <h2>{{ activity.name }}</h2>
                    <p>刷取次数：{{ activity.count }}</p>
                    <p>总体力消耗：{{ activity.consume }}</p>
                    <el-table :data="activityResources[activity.id]" style="width: 100%">
                        <el-table-column prop="resourceName" label="资源名称" />
                        <el-table-column prop="amount" label="数量" />
                    </el-table>
                </el-card>
            </div>
        </el-main>
    </el-container>
</template>

<script lang="ts">
import { ref, onMounted } from "vue"
import axios from "axios"
import { ElMessage } from "element-plus"

interface ResourceType {
    id: number
    name: string
}

interface Activity {
    id: number
    name: string
    count: number
    consume: number
}

interface Resource {
    resourceName: string
    amount: number
}

export default {
    name: "App",
    setup() {
        const newResourceType = ref("")
        const newActivity = ref("")
        const resourceTypes = ref<ResourceType[]>([])
        const activities = ref<Activity[]>([])
        const selectedActivity = ref<number | null>(null)
        const batchRecords = ref<{ resourceTypeId: number; resourceTypeName: string; amount: number }[]>([])
        const activityResources = ref<Record<number, Resource[]>>({})
        const staminaCost = ref<number>(0)

        const fetchResourceTypes = async () => {
            try {
                const { data } = await axios.get("/api/resource_types")
                resourceTypes.value = data
                batchRecords.value = data.map((type: ResourceType) => ({
                    resourceTypeId: type.id,
                    resourceTypeName: type.name,
                    amount: 0,
                }))
            } catch (error) {
                console.error("获取资源类型失败：", error)
            }
        }

        const fetchActivities = async () => {
            try {
                const { data } = await axios.get("/api/activities")
                activities.value = data
            } catch (error) {
                console.error("获取活动失败：", error)
            }
        }

        const fetchActivityResources = async (activityId: number) => {
            try {
                const { data } = await axios.get(`/api/activity_resources/${activityId}`)
                activityResources.value[activityId] = data.map((item: [string, number]) => ({
                    resourceName: item[0],
                    amount: item[1],
                }))
            } catch (error) {
                console.error(`获取活动 ${activityId} 的资源记录失败：`, error)
                activityResources.value[activityId] = []
            }
        }

        const addResourceType = async () => {
            if (!newResourceType.value.trim()) {
                alert("资源名称不能为空！")
                return
            }

            try {
                const response = await axios.post("/api/resource_types", {
                    name: newResourceType.value.trim(),
                })
                resourceTypes.value.push(response.data)
                batchRecords.value.push({
                    resourceTypeId: response.data.id,
                    resourceTypeName: response.data.name,
                    amount: 0,
                })
                newResourceType.value = ""
            } catch (error) {
                console.error("添加资源类型失败：", error)
                alert("添加资源类型失败，请重试！")
            }
        }

        const addActivity = async () => {
            if (!newActivity.value.trim()) {
                alert("活动名称不能为空！")
                return
            }

            try {
                const response = await axios.post("/api/activities", {
                    name: newActivity.value.trim(),
                    count: 0,
                    consume: 0,
                })
                activities.value.push(response.data)
                newActivity.value = ""
            } catch (error) {
                console.error("添加活动失败：", error)
                alert("添加活动失败，请重试！")
            }
        }

        const submitBatchRecords = async () => {
            if (!selectedActivity.value) {
                alert("请选择活动！")
                return
            }

            if (staminaCost.value <= 0) {
                alert("体力消耗必须大于 0！")
                return
            }

            const records = batchRecords.value
                .filter((record) => record.amount > 0)
                .map((record) => ({
                    type_id: record.resourceTypeId,
                    amount: record.amount,
                    activity_id: selectedActivity.value,
                }))

            if (records.length === 0) {
                ElMessage("没有需要提交的记录！")
                return
            }

            try {
                await axios.post("/api/batch_resources", {
                    records,
                    stamina_cost: staminaCost.value,
                })
                ElMessage.success("批量记录提交成功！")

                // 清空所有资源的数量
                batchRecords.value = batchRecords.value.map((record) => ({
                    ...record,
                    amount: 0, // 重置数量为 0
                }))
                fetchActivities()
                fetchActivityResources(selectedActivity.value)
            } catch (error) {
                console.error("批量提交记录失败：", error)
                alert("批量提交记录失败，请重试！")
            }
        }

        onMounted(async () => {
            await fetchResourceTypes()
            await fetchActivities()
            for (const activity of activities.value) {
                await fetchActivityResources(activity.id)
            }
        })

        return {
            newResourceType,
            newActivity,
            resourceTypes,
            activities,
            selectedActivity,
            batchRecords,
            activityResources,
            staminaCost,
            addResourceType,
            addActivity,
            submitBatchRecords,
        }
    },
}
</script>

<style scoped>
.box-card {
    margin: 20px 0;
}

.input-with-button {
    margin-right: 10px;
    width: 300px;
}

.activity-section {
    margin-bottom: 30px;
}
</style>