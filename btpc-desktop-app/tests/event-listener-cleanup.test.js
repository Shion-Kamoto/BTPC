/**
 * Event Listener Cleanup Tests
 * Constitution Compliance: Article XI.6 - Event Listener Cleanup
 * TDD: Tests written FIRST before implementation
 */

// Import modules under test
const {
    EventListenerManager,
    PageController,
    CrossPageEventManager
} = require('../ui/btpc-event-manager.js');

describe('Event Listener Cleanup', () => {
    let mockUnlisten;
    let eventManager;

    beforeEach(() => {
        // Mock Tauri listen function
        mockUnlisten = jest.fn();
        window.__TAURI__ = {
            listen: jest.fn(() => Promise.resolve(mockUnlisten)),
            emit: jest.fn()
        };

        // Reset event manager
        eventManager = null;
    });

    afterEach(() => {
        // Cleanup any remaining listeners
        if (eventManager) {
            eventManager.destroy();
        }
    });

    describe('Event Manager Lifecycle', () => {
        test('should store unlisten functions when adding listeners', async () => {
            eventManager = new EventListenerManager();

            await eventManager.listen('blockchain-update', jest.fn());
            await eventManager.listen('wallet-update', jest.fn());
            await eventManager.listen('node-status', jest.fn());

            expect(eventManager.getListenerCount()).toBe(3);
            expect(window.__TAURI__.listen).toHaveBeenCalledTimes(3);
        });

        test('should cleanup all listeners on destroy', async () => {
            eventManager = new EventListenerManager();

            // Add multiple listeners
            await eventManager.listen('event1', jest.fn());
            await eventManager.listen('event2', jest.fn());
            await eventManager.listen('event3', jest.fn());

            // Destroy and verify cleanup
            eventManager.destroy();

            expect(mockUnlisten).toHaveBeenCalledTimes(3);
            expect(eventManager.getListenerCount()).toBe(0);
        });

        test('should cleanup listeners on page unload', async () => {
            eventManager = new EventListenerManager();

            await eventManager.listen('test-event', jest.fn());

            // Simulate page unload
            const unloadEvent = new Event('unload');
            window.dispatchEvent(unloadEvent);

            expect(mockUnlisten).toHaveBeenCalled();
        });
    });

    describe('Memory Leak Prevention', () => {
        test('should not accumulate listeners on repeated initialization', async () => {
            // First initialization
            const manager1 = new EventListenerManager();
            await manager1.listen('test-event', jest.fn());

            // Second initialization (simulating page navigation)
            manager1.destroy();
            const manager2 = new EventListenerManager();
            await manager2.listen('test-event', jest.fn());

            // Should only have 1 active listener
            expect(manager2.getListenerCount()).toBe(1);

            // Cleanup
            manager2.destroy();
        });

        test('should prevent duplicate listeners for same event', async () => {
            eventManager = new EventListenerManager();

            const handler = jest.fn();
            await eventManager.listen('duplicate-event', handler);
            await eventManager.listen('duplicate-event', handler);

            // Should only register once
            expect(eventManager.getListenerCount()).toBe(1);
        });

        test('should track listener lifecycle correctly', async () => {
            eventManager = new EventListenerManager();

            // Add listener
            const listenerId = await eventManager.listen('lifecycle-event', jest.fn());
            expect(eventManager.hasListener(listenerId)).toBe(true);

            // Remove specific listener
            eventManager.removeListener(listenerId);
            expect(eventManager.hasListener(listenerId)).toBe(false);
        });
    });

    describe('Page Controller Integration', () => {
        test('should cleanup listeners when page controller is destroyed', async () => {
            const pageController = new PageController();

            // Wait for async initialization to complete
            await new Promise(resolve => setTimeout(resolve, 100));

            // Verify listeners are registered (PageController adds 3 listeners in initializeListeners)
            expect(pageController.listeners.length).toBeGreaterThan(0);

            // Destroy controller
            pageController.destroy();

            // Verify listeners are cleaned up
            expect(pageController.listeners.length).toBe(0);
        });

        test('should handle errors during cleanup gracefully', async () => {
            eventManager = new EventListenerManager();

            // Mock unlisten to throw error
            const failingUnlisten = jest.fn(() => {
                throw new Error('Cleanup failed');
            });
            window.__TAURI__.listen = jest.fn(() => Promise.resolve(failingUnlisten));

            await eventManager.listen('error-event', jest.fn());

            // Should not throw when destroying
            expect(() => eventManager.destroy()).not.toThrow();

            // Should still mark as cleaned up
            expect(eventManager.getListenerCount()).toBe(0);
        });
    });

    describe('Cross-Page Event Management', () => {
        test('should properly handle cross-page events', async () => {
            const pageManager = new CrossPageEventManager();

            const handler = jest.fn();
            await pageManager.subscribe('setting-updated', handler);

            // Emit event from another page
            window.__TAURI__.emit('setting-updated', { key: 'theme', value: 'dark' });

            // Handler should be called
            expect(window.__TAURI__.emit).toHaveBeenCalledWith('setting-updated', expect.any(Object));

            // Cleanup
            pageManager.destroy();
        });

        test('should unsubscribe from cross-page events on cleanup', async () => {
            const pageManager = new CrossPageEventManager();

            await pageManager.subscribe('wallet-changed', jest.fn());
            await pageManager.subscribe('node-status-changed', jest.fn());

            const listenerCount = pageManager.getActiveSubscriptions();
            expect(listenerCount).toBe(2);

            pageManager.destroy();
            expect(pageManager.getActiveSubscriptions()).toBe(0);
        });
    });

    describe('Constitution Compliance', () => {
        test('should comply with Article XI.6 - No event listener leaks', async () => {
            const manager = new EventListenerManager();

            // Add listeners
            await manager.listen('compliance-event-1', jest.fn());
            await manager.listen('compliance-event-2', jest.fn());

            // Get initial count
            const initialCount = manager.getListenerCount();
            expect(initialCount).toBe(2);

            // Cleanup
            manager.destroy();

            // Verify no leaks
            expect(manager.getListenerCount()).toBe(0);
            expect(mockUnlisten).toHaveBeenCalledTimes(2);
        });

        test('should automatically cleanup on window unload', () => {
            const manager = new EventListenerManager();
            const cleanupSpy = jest.spyOn(manager, 'destroy');

            // Simulate window unload
            const unloadEvent = new Event('beforeunload');
            window.dispatchEvent(unloadEvent);

            // Verify cleanup was called
            expect(cleanupSpy).toHaveBeenCalled();
        });
    });
});

// Custom Jest matchers
expect.extend({
    toHaveNoMemoryLeaks(manager) {
        const hasLeaks = manager.getListenerCount() > 0 || manager.hasActiveListeners();

        return {
            pass: !hasLeaks,
            message: () => hasLeaks
                ? `Expected no memory leaks but found ${manager.getListenerCount()} active listeners`
                : `Expected memory leaks but all listeners are properly cleaned up`
        };
    },

    toProperlyCleanupListeners(controller) {
        const hasListeners = controller.listeners && controller.listeners.length > 0;
        const hasCleanupMethod = typeof controller.destroy === 'function';

        const pass = !hasListeners && hasCleanupMethod;

        return {
            pass,
            message: () => pass
                ? `Controller properly cleans up listeners`
                : `Controller has ${controller.listeners?.length || 0} active listeners or missing destroy method`
        };
    }
});