import { watch } from 'vue';

export function useMenuCoordination(menus, options = {}) {
  const { allowTransitions = [] } = options;

  for (const [currentMenu, currentRef] of Object.entries(menus)) {
    watch(currentRef, (newValue) => {
      if (newValue) {
        const lastOpenMenu = Object.entries(menus).find(
          ([_, ref]) => ref.value && ref !== currentRef
        )?.[0];

        for (const [otherMenu, otherRef] of Object.entries(menus)) {
          if (otherMenu !== currentMenu) {
            const shouldAllowTransition = allowTransitions.some(
              (transition) =>
                transition.from === lastOpenMenu && transition.to.includes(currentMenu)
            );

            if (!shouldAllowTransition) {
              otherRef.value = false;
            }
          }
        }
      }
    });
  }
}
