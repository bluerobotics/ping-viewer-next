<template>
  <div class="notification-container">
    <v-btn
      icon
      class="notification-button"
      :class="{ 'has-notifications': hasUnreadNotifications }"
      @click="showNotifications = !showNotifications"
    >
      <v-icon>mdi-bell</v-icon>
      <v-badge
        v-if="unreadCount > 0"
        :content="unreadCount"
        color="error"
        class="notification-badge"
      />
    </v-btn>

    <v-menu
      v-model="showNotifications"
      location="top end"
      :close-on-content-click="false"
      class="notification-menu"
    >
      <v-card min-width="300" max-width="400" max-height="500" class="notification-card">
        <v-card-title class="d-flex justify-space-between align-center">
          Notifications
          <v-btn
            v-if="notifications.length > 0"
            variant="text"
            size="small"
            @click="clearNotifications"
          >
            Clear All
          </v-btn>
        </v-card-title>

        <v-divider />

        <v-card-text class="notification-list pa-0">
          <div v-if="notifications.length === 0" class="text-center pa-4 text-medium-emphasis">
            No notifications
          </div>
          <v-list v-else>
            <v-list-item
              v-for="notification in notifications"
              :key="notification.id"
              :class="{ 'unread': !notification.read }"
            >
              <template v-slot:prepend>
                <v-icon :color="notification.color || 'primary'">
                  {{ notification.icon || 'mdi-information' }}
                </v-icon>
              </template>

              <v-list-item-title>{{ notification.title }}</v-list-item-title>
              <v-list-item-subtitle>{{ notification.message }}</v-list-item-subtitle>

              <template v-slot:append>
                <v-btn
                  icon="mdi-close"
                  variant="text"
                  size="small"
                  @click="removeNotification(notification.id)"
                />
              </template>
            </v-list-item>
          </v-list>
        </v-card-text>
      </v-card>
    </v-menu>
  </div>
</template>

<script setup>
import { useNotificationStore } from '@/stores/notificationStore';
import { computed, ref } from 'vue';

const notificationStore = useNotificationStore();
const showNotifications = ref(false);

const notifications = computed(() => notificationStore.notifications);
const unreadCount = computed(() => notifications.value.filter((n) => !n.read).length);
const hasUnreadNotifications = computed(() => unreadCount.value > 0);

const removeNotification = (id) => {
  notificationStore.removeNotification(id);
};

const clearNotifications = () => {
  notificationStore.clearNotifications();
};
</script>

<style scoped>
.notification-container {
  position: relative;
}

.notification-button {
  position: relative;
}

.notification-badge {
  position: absolute;
  top: 0;
  right: 0;
}

.notification-card {
  overflow: hidden;
}

.notification-list {
  overflow-y: auto;
}

.notification-list .v-list-item {
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.12);
}

.notification-list .v-list-item:last-child {
  border-bottom: none;
}

.notification-list .v-list-item.unread {
  background-color: rgba(var(--v-theme-primary), 0.05);
}

.notification-list .v-list-item.unread::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 4px;
  background-color: rgb(var(--v-theme-primary));
}
</style> 