<template>
  <div class="notification-menu">
    <div class="notification-list mb-4">
      <template v-if="notifications.length === 0">
        <div class="empty-state">
          <v-icon size="48" class="mb-2">mdi-bell-off</v-icon>
          <div>No notifications</div>
        </div>
      </template>

      <v-list density="compact">
        <v-list-item v-for="notification in sortedNotifications" :key="notification.id"
          :class="{ 'unread': !notification.read }"
          @click="markAsRead(notification.id)">
          <template v-slot:prepend>
            <div class="text-caption mr-2">{{ formatTime(notification.timestamp) }}</div>
          </template>

          <v-list-item-title>{{ notification.title }}</v-list-item-title>
          <v-list-item-subtitle class="d-flex align-center gap-2">
            <v-chip
              size="x-small"
              variant="outlined"
              class="text-caption"
              :title="notification.message.split('device')[1]"
            >
              {{ notification.device_type }}
            </v-chip>
          </v-list-item-subtitle>

          <template v-slot:append>
            <v-btn
              icon="mdi-close"
              variant="text"
              size="x-small"
              @click.stop="removeNotification(notification.id)"
            />
          </template>
        </v-list-item>
      </v-list>
    </div>

    <v-divider class="my-2"></v-divider>

    <div class="d-flex justify-space-between align-center px-4 py-2">
      <v-btn
        v-if="unreadCount > 0"
        size="small"
        variant="text"
        @click="markAllAsRead"
        class="text-caption"
      >
        Mark All Read
      </v-btn>
      <v-btn
        v-if="notifications.length > 0"
        size="small"
        variant="text"
        color="error"
        @click="clearAll"
        class="text-caption"
      >
        Clear All
      </v-btn>
    </div>
  </div>
</template>

<script setup>
import { useNotificationStore } from '@/stores/notificationStore';
import { computed, ref, watch } from 'vue';

const props = defineProps({
  glass: {
    type: Boolean,
    default: false,
  },
  iconSize: {
    type: String,
    default: 'default',
  },
  isOpen: {
    type: Boolean,
    default: false,
  },
});

const emit = defineEmits(['update:is-open']);

const notificationStore = useNotificationStore();
const notifications = computed(() => notificationStore.notifications);
const unreadCount = computed(() => notifications.value.filter((n) => !n.read).length);

// Sorting state
const sortKey = ref('timestamp');
const sortOrder = ref('desc');

// Sort notifications based on current sort key and order
const sortedNotifications = computed(() => {
  return [...notifications.value].sort((a, b) => {
    let comparison = 0;

    if (sortKey.value === 'timestamp') {
      comparison = a.timestamp - b.timestamp;
    } else if (sortKey.value === 'device') {
      const deviceA = a.message.split('device ')[1] || '';
      const deviceB = b.message.split('device ')[1] || '';
      comparison = deviceA.localeCompare(deviceB);
    } else if (sortKey.value === 'title') {
      comparison = a.title.localeCompare(b.title);
    }

    return sortOrder.value === 'asc' ? comparison : -comparison;
  });
});

const sortBy = (key) => {
  if (sortKey.value === key) {
    sortOrder.value = sortOrder.value === 'asc' ? 'desc' : 'asc';
  } else {
    sortKey.value = key;
    sortOrder.value = 'asc';
  }
};

const formatTime = (timestamp) => {
  const date = new Date(timestamp);
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
};

const markAsRead = (id) => {
  notificationStore.markAsRead(id);
};

const markAllAsRead = () => {
  notificationStore.markAllAsRead();
};

const removeNotification = (id) => {
  notificationStore.removeNotification(id);
};

const clearAll = () => {
  notificationStore.clearNotifications();
};

watch(
  () => props.isOpen,
  (newValue) => {
    emit('update:is-open', newValue);
  }
);
</script>

<style scoped>
.notification-menu {
  width: 400px;
  max-width: calc(100vw - var(--button-size));
  height: 400px;
  display: flex;
  flex-direction: column;
}

.notification-list {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  position: relative;
}

.empty-state {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  text-align: center;
  color: rgba(var(--v-theme-on-surface), 0.6);
}

.notification-list :deep(.v-list) {
  background: transparent;
}

.notification-list :deep(.v-list-item) {
  min-height: 40px;
  padding: 4px 8px;
}

.notification-list :deep(.v-list-item.unread) {
  background-color: rgba(var(--v-theme-on-surface), 0.05);
}

.notification-list :deep(.v-list-item.unread)::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 4px;
  background-color: rgb(var(--v-theme-on-surface));
  opacity: 0.6;
}

.notification-list :deep(.v-list-item:hover) {
  background-color: rgba(var(--v-theme-on-surface), 0.04);
}

@media (max-width: 600px) {
  .notification-menu {
    width: calc(100vw - var(--button-size) - var(--button-gap) * 2);
    max-width: 400px;
    height: 350px;
  }
}
</style>