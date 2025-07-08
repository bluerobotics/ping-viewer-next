import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useNotificationStore = defineStore('notifications', () => {
  const notifications = ref([]);
  const nextId = ref(1);

  const addNotification = (notification) => {
    const id = nextId.value++;
    notifications.value.push({
      id,
      ...notification,
      read: false,
      timestamp: new Date(),
      device_type: notification.device_type || 'unknown',
    });
  };

  const removeNotification = (id) => {
    const index = notifications.value.findIndex((n) => n.id === id);
    if (index !== -1) {
      notifications.value.splice(index, 1);
    }
  };

  const clearNotifications = () => {
    notifications.value = [];
  };

  const markAsRead = (id) => {
    const notification = notifications.value.find((n) => n.id === id);
    if (notification) {
      notification.read = true;
    }
  };

  const markAllAsRead = () => {
    for (const notification of notifications.value) {
      notification.read = true;
    }
  };

  return {
    notifications,
    addNotification,
    removeNotification,
    clearNotifications,
    markAsRead,
    markAllAsRead,
  };
});
