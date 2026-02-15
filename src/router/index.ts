import { createRouter, createWebHistory } from "vue-router";
import Dashboard from "@/views/Dashboard.vue";
import McpDetail from "@/views/McpDetail.vue";
import AddMcp from "@/views/AddMcp.vue";
import Settings from "@/views/Settings.vue";

const routes = [
  {
    path: "/",
    name: "Dashboard",
    component: Dashboard,
  },
  {
    path: "/mcp/:id",
    name: "McpDetail",
    component: McpDetail,
    props: true,
  },
  {
    path: "/add",
    name: "AddMcp",
    component: AddMcp,
  },
  {
    path: "/settings",
    name: "Settings",
    component: Settings,
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
